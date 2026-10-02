use std::{format, fs::File, io::Read, path::Path, process::{Command, Stdio}, sync::Arc, thread::{self, JoinHandle}, time::Duration, vec};

use parking_lot::Mutex;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ringbuf::{HeapProd, HeapRb, traits::{Producer, Split}};
use symphonia::core::{audio::Position, codecs::audio::AudioDecoderOptions, errors::Error::DecodeError, formats::{FormatOptions, TrackType, probe::Hint}, io::MediaSourceStream, meta::MetadataOptions};
use symphonium::resample::fixed_resample::rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use uuid::Uuid;

use crate::{common::{base::file::SaveableFile, constant::ENDIANESS, log}, server::ServerState};

pub struct ServerFile {
	pub base: SaveableFile,
	lock: Arc<Mutex<()>>,
	path: String
}

impl ServerFile {
	pub fn new(path: String, server_state: &ServerState) -> Self {
		let lock = if server_state.config.playlist_mode {
			server_state.playlist_lock.clone()
		} else {
			Arc::new(Mutex::new(()))
		};

		Self::new_with_lock(path, lock, server_state)
	}

	pub fn new_with_lock(path: String, lock: Arc<Mutex<()>>, server_state: &ServerState) -> Self {
		let pathed = Path::new(&path);
		let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
		let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
		let base = match server_state.config.files.get(&parent) {
			Some(map) => {
				match map.get(&name) {
					Some(entry) => entry.clone(),
					None => SaveableFile::default(),
				}
			},
			None => SaveableFile::default()
		};

		Self { base, lock, path }
	}

	pub fn play(&self, server_state: &mut ServerState) -> Option<JoinHandle<()>> {
		let uuid = Uuid::new_v4();
		let (sample_rate, prod) = {
			let sample_rate = server_state.sample_rate as usize;
			let rb = HeapRb::<f32>::new(sample_rate / 16);
			let (prod, cons) = rb.split();
			server_state.audio_data.insert(uuid, Arc::new(Mutex::new(cons)));
			(sample_rate, Arc::new(Mutex::new(prod)))
		};

		// Try Symphonia
		let thread = match self.play_with_symphonia(sample_rate, prod.clone(), self.lock.clone()) {
			Ok(thread) => thread,
			Err(err) => {
				// Try FFMPEG
				log::warn(format!("Failed to play file with symphonia: {:?}", err));
				match self.play_with_ffmpeg(sample_rate, prod, self.lock.clone()) {
					Ok(thread) => thread,
					Err(err) => {
						log::error(format!("Failed to play file with ffmpeg: {:?}", err));
						server_state.audio_data.remove(&uuid);
						return None;
					}
				}
			}
		};

		Some(thread)
	}

	fn play_with_symphonia(&self, sample_rate: usize, prod: Arc<Mutex<HeapProd<f32>>>, lock: Arc<Mutex<()>>) -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
    let file = Box::new(File::open(Path::new(&self.path))?);
		let mss = MediaSourceStream::new(file, Default::default());
    let hint = Hint::new();

    // Use the default options when reading and decoding.
    let fmt_opts = FormatOptions::default();
    let meta_opts = MetadataOptions::default();
    let dec_opts = AudioDecoderOptions::default();

    // Probe the media source stream for a format.
    let mut format = symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts)?;
    // Get the default audio track.
    let track = format.default_track(TrackType::Audio).unwrap();
    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
      .make_audio_decoder(track.codec_params.as_ref().unwrap().audio().unwrap(), &dec_opts)?;
			
    // Store the track identifier, we'll use it to filter packets.
    let track_id = track.id;

		Ok(thread::spawn(move || {
			let _locked = lock.lock();
			let mut prod = prod.lock();
			let mut channel_samples: Vec<Vec<f32>> = vec![];
	    while let Some(packet) = format.next_packet().unwrap() {
        // If the packet does not belong to the selected track, skip it.
        if packet.track_id != track_id {
          continue;
        }

        // Decode the packet into audio samples, ignoring any decode errors.
        match decoder.decode(&packet) {
          Ok(audio_buf) => {
						let rate = audio_buf.spec().rate() as usize;
						let channels = audio_buf.spec().channels();
						let frames = audio_buf.frames();

						audio_buf.copy_to_vecs_planar(&mut channel_samples);

						// Force stereo
						match channels.count() {
							1 => {
								let samples = channel_samples[0].clone();
								channel_samples.push(samples);
							},
							2 => (),
							_ => {
								let left = match channels.get_canonical_index_for_positioned_channel(Position::FRONT_LEFT) {
									Some(index) => index,
									None => 0
								};
								let right = match channels.get_canonical_index_for_positioned_channel(Position::FRONT_RIGHT) {
									Some(index) => index,
									None => 1
								};
								let left = channel_samples[left].clone();
								let right = channel_samples[right].clone();
								channel_samples = vec![left, right];
							}
						}

						let interleaved = if sample_rate != rate {
							// Resample
							let params = SincInterpolationParameters {
				        sinc_len: 256,
				        f_cutoff: 0.95,
				        interpolation: SincInterpolationType::Linear,
				        oversampling_factor: 256,
				        window: WindowFunction::BlackmanHarris2,
							};

							let mut resampler = match SincFixedIn::<f32>::new(sample_rate as f64 / rate as f64, 2.0, params, frames, 2) {
								Ok(resampler) => resampler,
								Err(err) => {
									log::error(format!("Failed to create resampler: {:?}", err));
									break;
								}
							};

							let resampled = match resampler.process(&channel_samples, None) {
								Ok(resampled) => resampled,
								Err(err) => {
									log::error(format!("Failed to resample: {:?}", err));
									break;
								}
							};

							resampled[0].par_iter().zip(resampled[1].par_iter()).flat_map(|(left, right)| [*left, *right]).collect::<Vec<_>>()
						} else {
							channel_samples[0].par_iter().zip(channel_samples[1].par_iter()).flat_map(|(left, right)| [*left, *right]).collect::<Vec<_>>()
						};
						let mut offset = prod.push_slice(&interleaved);
						while offset < interleaved.len() {
							thread::sleep(Duration::from_millis(10));
							offset += prod.push_slice(&interleaved[offset..]);
						}
          }
          Err(DecodeError(err)) => log::warn(format!("Symphonia decode error: {}", err)),
          Err(err) => {
						log::error(format!("Symphonia error: {:?}", err));
						break;
					},
        }
	    }
		}))
	}

	fn play_with_ffmpeg(&self, sample_rate: usize, prod: Arc<Mutex<HeapProd<f32>>>, lock: Arc<Mutex<()>>) -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
		let mut result = Command::new("ffmpeg").args([
			"-loglevel", "-8",
			"-i", &self.path,
			"-f", format!("f32{}", ENDIANESS).as_str(),
			"-ac", "2",
			"-ar", sample_rate.to_string().as_str(),
			"-"
		]).stdout(Stdio::piped()).spawn()?;
		let mut stdout = result.stdout.take().unwrap();
		Ok(thread::spawn(move || {
			let _locked = lock.lock();
			let mut prod = prod.lock();
			let mut buf_size = sample_rate * 4 / 16;
			while buf_size % 4 != 0 {
				buf_size += 1;
			}
			let mut buf = vec![0u8; buf_size];
			loop {
				match stdout.read(&mut buf) {
					Ok(read) => {
						if read == 0 {
							break;
						}

						let read = read / 4;
						let buf: &[f32] = bytemuck::cast_slice(&buf);
						let mut offset = prod.push_slice(&buf[..read]);
						while offset < read {
							thread::sleep(Duration::from_millis(100));
							offset += prod.push_slice(&buf[offset..read]);
						}
					},
					Err(err) => {
						log::error(format!("ffmpeg error: {:?}", err));
						break;
					}
				}
			}
		}))
	}
}