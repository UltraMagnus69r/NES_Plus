//! Tiny PCM WAV reader and writer. Ogg and MP3 go through the optional decoder.

pub fn wav_from_samples(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_size = samples.len() * 2;
    let mut buf = vec![0u8; 44 + data_size];
    buf[0..4].copy_from_slice(b"RIFF");
    write_u32(&mut buf, 4, (36 + data_size) as u32);
    buf[8..12].copy_from_slice(b"WAVE");
    buf[12..16].copy_from_slice(b"fmt ");
    write_u32(&mut buf, 16, 16);
    write_u16(&mut buf, 20, 1);
    write_u16(&mut buf, 22, 1);
    write_u32(&mut buf, 24, rate);
    write_u32(&mut buf, 28, rate * 2);
    write_u16(&mut buf, 32, 2);
    write_u16(&mut buf, 34, 16);
    buf[36..40].copy_from_slice(b"data");
    write_u32(&mut buf, 40, data_size as u32);
    for (i, sample) in samples.iter().enumerate() {
        let s = sample.clamp(-1.0, 1.0);
        let v = if s < 0.0 { s * 32768.0 } else { s * 32767.0 };
        write_i16(&mut buf, 44 + i * 2, v as i16);
    }
    buf
}

pub fn read_pcm_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE file".into());
    }
    let mut pos = 12usize;
    let mut audio_format = 1u16;
    let mut channels = 1u16;
    let mut rate = 44100u32;
    let mut bits = 16u16;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end >= start + 16 {
            audio_format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        pos = start + size + (size & 1);
    }
    let data = data.ok_or("WAVE file has no data chunk")?;
    if audio_format != 1 {
        return Err("only PCM WAV is read here; try the compressed decoder for other encodings".into());
    }
    if channels == 0 {
        return Err("WAV has zero channels".into());
    }
    let mut samples = Vec::new();
    match bits {
        16 => {
            let frames = data.len() / 2 / channels as usize;
            for frame in 0..frames {
                let mut acc = 0.0f32;
                for ch in 0..channels as usize {
                    let o = (frame * channels as usize + ch) * 2;
                    if o + 2 > data.len() {
                        break;
                    }
                    let v = i16::from_le_bytes([data[o], data[o + 1]]);
                    acc += v as f32 / 32768.0;
                }
                samples.push(acc / channels as f32);
            }
        }
        8 => {
            let frames = data.len() / channels as usize;
            for frame in 0..frames {
                let mut acc = 0.0f32;
                for ch in 0..channels as usize {
                    let o = frame * channels as usize + ch;
                    if o >= data.len() {
                        break;
                    }
                    acc += (data[o] as f32 - 128.0) / 128.0;
                }
                samples.push(acc / channels as f32);
            }
        }
        _ => return Err(format!("{bits}-bit PCM is not supported; use 8 or 16 bit")),
    }
    if samples.is_empty() {
        return Err("WAV data is empty".into());
    }
    Ok((samples, rate.max(1)))
}

pub fn import_audio(bytes: &[u8], filename: &str) -> Result<(Vec<f32>, u32), String> {
    if let Ok(wav) = read_pcm_wav(bytes) {
        return Ok(wav);
    }
    #[cfg(feature = "decode")]
    {
        return decode_symphonia(bytes, filename);
    }
    #[cfg(not(feature = "decode"))]
    {
        let _ = filename;
        Err("this build reads PCM WAV. Ogg and MP3 need the decode feature".into())
    }
}

#[cfg(feature = "decode")]
fn decode_symphonia(bytes: &[u8], filename: &str) -> Result<(Vec<f32>, u32), String> {
    use std::io::Cursor;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = filename.rsplit('.').next() {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("could not read audio: {e}"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or("audio file has no default track")?
        .clone();
    let rate = track.codec_params.sample_rate.unwrap_or(44100);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("could not open audio decoder: {e}"))?;
    let mut samples = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track.id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);
        let ch = spec.channels.count().max(1);
        for frame in buf.samples().chunks(ch) {
            let acc: f32 = frame.iter().copied().sum();
            samples.push(acc / ch as f32);
        }
        if samples.len() > 16000 * 30 {
            break;
        }
    }
    if samples.is_empty() {
        return Err("decoder produced no samples".into());
    }
    Ok((samples, rate))
}

fn write_u16(buf: &mut [u8], at: usize, v: u16) {
    buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn write_i16(buf: &mut [u8], at: usize, v: i16) {
    buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn write_u32(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

pub fn synth_tone(freq: f32, seconds: f32, rate: u32, vol: f32, decay: f32) -> Vec<f32> {
    let n = ((seconds * rate as f32).max(1.0)) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / rate as f32;
        let env = (-decay * t).exp();
        let s = (t * freq * std::f32::consts::TAU).sin();
        let harm = (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.18;
        out.push((s + harm) * env * vol);
    }
    out
}

pub fn synth_sweep(start: f32, end: f32, seconds: f32, rate: u32, vol: f32) -> Vec<f32> {
    let n = ((seconds * rate as f32).max(1.0)) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let freq = start + (end - start) * t;
        phase += freq / rate as f32;
        let env = (1.0 - t).powi(2);
        out.push((phase * std::f32::consts::TAU).sin() * env * vol);
    }
    out
}
