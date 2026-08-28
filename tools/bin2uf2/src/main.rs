//! Converts a raw firmware binary to UF2
//!
//! cargo-hex-to-uf2 only supports a fixed list of family ids and the agar bootloader's 0x9d5bcf10
//! is not supported

use std::{env, fs, process};

const MAGIC_START0: u32 = 0x0A32_4655;
const MAGIC_START1: u32 = 0x9E5D_5157;
const MAGIC_END: u32 = 0x0AB1_6F30;
// the last header word is a family id
const FLAG_FAMILY_ID: u32 = 0x2000;
const PAYLOAD: usize = 256;
const DATA: usize = 476;

struct Args {
    input: String,
    output: String,
    base: u32,
    family: u32,
}

fn main() {
    let args = parse_args().unwrap_or_else(|e| fail(&e));

    if !args.base.is_multiple_of(PAYLOAD as u32) {
        fail(&format!(
            "--base {:#x} is not a multiple of {PAYLOAD}",
            args.base
        ));
    }

    let data = fs::read(&args.input).unwrap_or_else(|e| fail(&format!("{}: {e}", args.input)));
    if data.is_empty() {
        fail(&format!("{} is empty", args.input));
    }

    let blocks = data.len().div_ceil(PAYLOAD);
    let mut uf2 = Vec::with_capacity(blocks * (32 + DATA + 4));
    for (i, chunk) in data.chunks(PAYLOAD).enumerate() {
        let header = [
            MAGIC_START0,
            MAGIC_START1,
            FLAG_FAMILY_ID,
            args.base + (i * PAYLOAD) as u32,
            // a short tail is zero-padded
            PAYLOAD as u32,
            i as u32,
            blocks as u32,
            args.family,
        ];
        for word in header {
            uf2.extend_from_slice(&word.to_le_bytes());
        }
        uf2.extend_from_slice(chunk);
        uf2.resize(uf2.len() + DATA - chunk.len(), 0);
        uf2.extend_from_slice(&MAGIC_END.to_le_bytes());
    }

    fs::write(&args.output, &uf2).unwrap_or_else(|e| fail(&format!("{}: {e}", args.output)));
    println!(
        "{}: {} bytes, {blocks} blocks at {:#x}",
        args.output,
        data.len(),
        args.base
    );
}

fn parse_args() -> Result<Args, String> {
    let (mut input, mut output, mut base, mut family) = (None, None, None, None);
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} takes a value"))
                .and_then(|v| parse_u32(&v).ok_or_else(|| format!("{name}: not a number: {v}")))
        };
        match arg.as_str() {
            "--base" => base = Some(value("--base")?),
            "--family" => family = Some(value("--family")?),
            _ if arg.starts_with('-') => return Err(format!("unknown flag: {arg}")),
            _ if input.is_none() => input = Some(arg),
            _ if output.is_none() => output = Some(arg),
            _ => return Err(format!("unexpected argument: {arg}")),
        }
    }
    Ok(Args {
        input: input.ok_or("missing input")?,
        output: output.ok_or("missing output")?,
        base: base.ok_or("missing --base")?,
        family: family.ok_or("missing --family")?,
    })
}

fn parse_u32(s: &str) -> Option<u32> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => s.parse().ok(),
    }
}

fn fail(message: &str) -> ! {
    eprintln!("bin2uf2: {message}");
    eprintln!("usage: bin2uf2 <input.bin> <output.uf2> --base <addr> --family <id>");
    process::exit(1);
}
