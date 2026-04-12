//! Read and decrypt Chrome cookies for a given domain on macOS.
//!
//! Chrome encrypts cookie values with AES-128-CBC using a key derived from
//! the "Chrome Safe Storage" Keychain entry via PBKDF2-SHA1.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Get the Chrome cookies database path.
fn chrome_cookies_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join("Library/Application Support/Google/Chrome/Default/Cookies")
}

/// Read the Chrome Safe Storage encryption key from macOS Keychain.
fn read_chrome_key() -> Result<Vec<u8>> {
    let output = Command::new("security")
        .args(["find-generic-password", "-w", "-s", "Chrome Safe Storage"])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        anyhow::bail!("Cannot read Chrome Safe Storage key from Keychain");
    }

    let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(password.into_bytes())
}

/// Derive AES-128 key from Chrome Safe Storage password using PBKDF2-SHA1.
fn derive_key(password: &[u8]) -> [u8; 16] {
    // Chrome uses: PBKDF2-SHA1, salt="saltysalt", iterations=1003, key_len=16
    let mut key = [0u8; 16];
    pbkdf2_sha1(password, b"saltysalt", 1003, &mut key);
    key
}

/// Minimal PBKDF2-SHA1 implementation (avoids adding heavy crypto crate).
fn pbkdf2_sha1(password: &[u8], salt: &[u8], iterations: u32, output: &mut [u8; 16]) {
    use std::num::Wrapping;

    // HMAC-SHA1
    fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
        let mut ipad_key = [0x36u8; 64];
        let mut opad_key = [0x5cu8; 64];
        let actual_key: Vec<u8> = if key.len() > 64 {
            sha1(key).to_vec()
        } else {
            key.to_vec()
        };
        for (i, &b) in actual_key.iter().enumerate() {
            ipad_key[i] ^= b;
            opad_key[i] ^= b;
        }
        let mut inner = ipad_key.to_vec();
        inner.extend_from_slice(data);
        let inner_hash = sha1(&inner);
        let mut outer = opad_key.to_vec();
        outer.extend_from_slice(&inner_hash);
        sha1(&outer)
    }

    fn sha1(data: &[u8]) -> [u8; 20] {
        let mut h0 = Wrapping(0x67452301u32);
        let mut h1 = Wrapping(0xEFCDAB89u32);
        let mut h2 = Wrapping(0x98BADCFEu32);
        let mut h3 = Wrapping(0x10325476u32);
        let mut h4 = Wrapping(0xC3D2E1F0u32);

        let ml = data.len() as u64 * 8;
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&ml.to_be_bytes());

        for chunk in msg.chunks(64) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
            }
            for i in 16..80 {
                w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
            }
            let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
            for i in 0..80 {
                let (f, k) = match i {
                    0..=19 => ((b & c) | ((!b) & d), Wrapping(0x5A827999)),
                    20..=39 => (b ^ c ^ d, Wrapping(0x6ED9EBA1)),
                    40..=59 => ((b & c) | (b & d) | (c & d), Wrapping(0x8F1BBCDC)),
                    _ => (b ^ c ^ d, Wrapping(0xCA62C1D6)),
                };
                let temp = Wrapping(a.0.rotate_left(5)) + f + e + k + Wrapping(w[i]);
                e = d; d = c; c = Wrapping(b.0.rotate_left(30)); b = a; a = temp;
            }
            h0 = h0 + a; h1 = h1 + b; h2 = h2 + c; h3 = h3 + d; h4 = h4 + e;
        }
        let mut result = [0u8; 20];
        result[0..4].copy_from_slice(&h0.0.to_be_bytes());
        result[4..8].copy_from_slice(&h1.0.to_be_bytes());
        result[8..12].copy_from_slice(&h2.0.to_be_bytes());
        result[12..16].copy_from_slice(&h3.0.to_be_bytes());
        result[16..20].copy_from_slice(&h4.0.to_be_bytes());
        result
    }

    // PBKDF2 with single block (output <= 20 bytes)
    let mut block_salt = salt.to_vec();
    block_salt.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha1(password, &block_salt);
    let mut result = u;
    for _ in 1..iterations {
        u = hmac_sha1(password, &u);
        for j in 0..20 {
            result[j] ^= u[j];
        }
    }
    output.copy_from_slice(&result[..16]);
}

/// Decrypt a Chrome v10 encrypted cookie value.
fn decrypt_v10(encrypted: &[u8], key: &[u8; 16]) -> Result<String> {
    if encrypted.len() < 3 || &encrypted[..3] != b"v10" {
        anyhow::bail!("Not a v10 encrypted value");
    }

    let data = &encrypted[3..];
    if data.len() < 16 || data.len() % 16 != 0 {
        anyhow::bail!("Invalid ciphertext length");
    }

    // AES-128-CBC with IV = 16 spaces
    let iv = [0x20u8; 16]; // ' ' * 16
    let plaintext = aes128_cbc_decrypt(data, key, &iv)?;

    // The first 32 bytes are noise (2 AES blocks), actual value starts at offset 32
    if plaintext.len() > 32 {
        Ok(String::from_utf8_lossy(&plaintext[32..]).to_string())
    } else {
        Ok(String::from_utf8_lossy(&plaintext).to_string())
    }
}

/// Minimal AES-128-CBC decryption with PKCS7 unpadding.
fn aes128_cbc_decrypt(ciphertext: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Result<Vec<u8>> {
    // Use OpenSSL CLI for decryption (available on all macOS)
    use std::io::Write;

    let mut cmd = Command::new("openssl")
        .args(["enc", "-d", "-aes-128-cbc", "-nopad",
               "-K", &hex::encode(key),
               "-iv", &hex::encode(iv)])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to run openssl")?;

    cmd.stdin.as_mut().unwrap().write_all(ciphertext)?;
    let output = cmd.wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!("openssl decrypt failed");
    }

    let mut plain = output.stdout;
    // PKCS7 unpad
    if let Some(&pad) = plain.last() {
        if (1..=16).contains(&pad) && plain.len() >= pad as usize {
            let pad_start = plain.len() - pad as usize;
            if plain[pad_start..].iter().all(|&b| b == pad) {
                plain.truncate(pad_start);
            }
        }
    }
    Ok(plain)
}

/// Simple hex encoding (avoids adding a crate).
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Read all Claude.ai cookies from Chrome's cookie database.
/// Returns a map of cookie_name → cookie_value.
pub fn read_claude_cookies() -> Result<HashMap<String, String>> {
    let db_path = chrome_cookies_path();
    if !db_path.exists() {
        anyhow::bail!("Chrome cookies database not found");
    }

    let chrome_password = read_chrome_key()?;
    let key = derive_key(&chrome_password);

    // Read cookies using sqlite3 CLI (avoids adding rusqlite dependency)
    let output = Command::new("sqlite3")
        .args([
            db_path.to_string_lossy().as_ref(),
            "SELECT name, hex(encrypted_value) FROM cookies WHERE host_key LIKE '%claude.ai%' AND name IN ('sessionKey','cf_clearance','lastActiveOrg','__ssid');",
        ])
        .output()
        .context("Failed to read Chrome cookies database")?;

    if !output.status.success() {
        anyhow::bail!("sqlite3 query failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut cookies = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            continue;
        }
        let name = parts[0].to_string();
        let hex_val = parts[1];

        // Decode hex to bytes
        let encrypted = match hex_to_bytes(hex_val) {
            Some(b) => b,
            None => continue,
        };

        if let Ok(value) = decrypt_v10(&encrypted, &key) {
            if !value.is_empty() {
                cookies.insert(name, value);
            }
        }
    }

    Ok(cookies)
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Generate a JavaScript snippet that sets cookies for claude.ai domain.
pub fn generate_cookie_injection_js() -> Option<String> {
    let cookies = read_claude_cookies().ok()?;

    if !cookies.contains_key("sessionKey") {
        return None;
    }

    let mut js_parts = Vec::new();
    for (name, value) in &cookies {
        // Escape single quotes in value
        let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
        js_parts.push(format!(
            "document.cookie = '{}={}; domain=.claude.ai; path=/; secure; SameSite=Lax';",
            name, escaped
        ));
    }

    Some(js_parts.join("\n"))
}
