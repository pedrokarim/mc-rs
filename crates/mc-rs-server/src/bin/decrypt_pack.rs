// Bedrock marketplace resource pack decryptor.
// Usage: decrypt_pack <pack.zip> <content_key_32chars> <out_dir>
//
// Format:
//   - The ZIP contains a `contents.json` file with a 256-byte header followed
//     by an AES-256-CFB8 encrypted JSON payload.
//   - The header layout :
//       [0..4]    magic 0x00 0x00 0x9B 0xFB
//       [4..5]    version (0x00)
//       [5..13]   reserved
//       [13..14]  uuid_len (typically 0x24)
//       [14..50]  pack UUID ASCII (36 chars)
//       [50..256] zero padding
//       [256..]   encrypted JSON
//   - The decrypted JSON has shape: { "version": N, "content": [ { "path": "...", "key": "..." }, ... ] }
//   - Each listed file is encrypted with its own 32-byte key when `key` is set.
//   - Files NOT listed (manifest.json, pack_icon.png, etc.) are copied as-is.
//
// AES-256-CFB8: key = ContentKey bytes (32), IV = ContentKey[0..16].

use std::io::Read;
use std::path::Path;

use aes::Aes256;
use cfb8::cipher::{AsyncStreamCipher, KeyIvInit};
use cfb8::Decryptor;

fn decrypt_cfb8(data: &mut [u8], key: &[u8], iv: &[u8]) {
    let dec = Decryptor::<Aes256>::new(key.into(), iv.into());
    dec.decrypt(data);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: decrypt_pack <pack.zip> <content_key_32chars> <out_dir>");
        std::process::exit(1);
    }
    let pack_path = &args[1];
    let key_str = &args[2];
    let out_dir = &args[3];

    let key_bytes = key_str.as_bytes();
    if key_bytes.len() != 32 {
        eprintln!(
            "content_key must be exactly 32 ASCII chars (got {})",
            key_bytes.len()
        );
        std::process::exit(1);
    }
    let iv: &[u8] = &key_bytes[..16];

    let file = std::fs::File::open(pack_path).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");

    std::fs::create_dir_all(out_dir).expect("mkdir out");

    // Step 1 : read raw bytes of every file into memory, write unencrypted ones
    // straight to disk. Keep encrypted blobs in a map for step 2.
    let mut raw_blobs: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let name = f.name().to_string();
        if f.is_dir() {
            continue;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).unwrap();
        raw_blobs.insert(name, buf);
    }

    // Step 2 : decrypt contents.json.
    let contents_raw = raw_blobs
        .remove("contents.json")
        .expect("no contents.json in pack");
    if contents_raw.len() < 0x100 {
        eprintln!(
            "contents.json too small ({} bytes), expected >= 256",
            contents_raw.len()
        );
        std::process::exit(1);
    }
    let header = &contents_raw[..0x100];
    println!("contents.json header magic = {:02X?}", &header[..4]);
    let uuid_len = header[0x0D] as usize;
    let uuid_str = String::from_utf8_lossy(&header[0x0E..0x0E + uuid_len]);
    println!("contents.json embedded UUID = {}", uuid_str);

    let mut payload = contents_raw[0x100..].to_vec();
    decrypt_cfb8(&mut payload, key_bytes, iv);

    // Some payloads have trailing zero padding after the JSON. Trim to last `}`.
    let last_brace = payload
        .iter()
        .rposition(|&b| b == b'}')
        .unwrap_or(payload.len() - 1);
    let trimmed = &payload[..=last_brace];
    let json_str = String::from_utf8_lossy(trimmed).into_owned();
    let out_contents = Path::new(out_dir).join("contents.json");
    std::fs::write(&out_contents, &json_str).unwrap();
    println!(
        "Wrote decrypted contents.json ({} bytes) → {:?}",
        json_str.len(),
        out_contents
    );

    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("parse contents.json");
    let content_list = parsed["content"]
        .as_array()
        .expect("no `content` array in contents.json");

    // Step 3 : iterate content list, decrypt each file that has a `key`.
    let mut keyed_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut count_encrypted = 0;
    let mut count_plain = 0;
    for entry in content_list {
        let path = entry["path"].as_str().unwrap_or("").to_string();
        if path.is_empty() {
            continue;
        }
        keyed_paths.insert(path.clone());

        let mut raw = match raw_blobs.remove(&path) {
            Some(b) => b,
            None => {
                eprintln!("warn: contents.json lists {} but not in zip", path);
                continue;
            }
        };

        if let Some(file_key) = entry.get("key").and_then(|k| k.as_str()) {
            let fk = file_key.as_bytes();
            if fk.len() != 32 {
                eprintln!("warn: invalid key length for {}", path);
                continue;
            }
            let fiv = &fk[..16];
            decrypt_cfb8(&mut raw, fk, fiv);
            count_encrypted += 1;
        } else {
            count_plain += 1;
        }

        let out = Path::new(out_dir).join(&path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(out, raw).unwrap();
    }

    // Step 4 : write remaining files NOT listed in contents.json (manifest.json, pack_icon.png).
    let mut count_extra = 0;
    for (path, data) in raw_blobs {
        if keyed_paths.contains(&path) {
            continue;
        }
        let out = Path::new(out_dir).join(&path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(out, data).unwrap();
        count_extra += 1;
    }

    println!(
        "Done : {} encrypted + {} plain (listed) + {} extra → {}",
        count_encrypted, count_plain, count_extra, out_dir
    );
}
