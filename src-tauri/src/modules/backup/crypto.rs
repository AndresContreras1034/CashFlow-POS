//! Cifrado de backups: Argon2id + AES-256-GCM por bloques.
//! Cabecera: "CFPOSENC" | versión(1) | sal(16) | prefijo nonce(7) | bloque u32 LE.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::stream::{DecryptorBE32, EncryptorBE32};
use aes_gcm::aead::{KeyInit, OsRng};
use aes_gcm::Aes256Gcm;
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"CFPOSENC";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const PREFIX_LEN: usize = 7;
const HEADER_LEN: usize = 8 + 1 + SALT_LEN + PREFIX_LEN + 4;
const CHUNK: u32 = 1 << 20;
const MAX_CHUNK: u32 = 64 << 20;
const TAG_LEN: usize = 16;

// Parámetros fijos de la versión 1 del formato: cambiarlos rompería los
// backups existentes (habría que subir VERSION).
const ARGON_MEMORY_KIB: u32 = 64 * 1024;
const ARGON_PASSES: u32 = 3;

const NOT_ENCRYPTED: &str = "El archivo no es un backup cifrado de CashFlow POS";
const BAD_DATA: &str = "Contraseña incorrecta o el archivo está dañado";

fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    let params = Params::new(ARGON_MEMORY_KIB, ARGON_PASSES, 1, Some(32))
        .map_err(|e| format!("Parámetros de cifrado inválidos: {e}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, key.as_mut_slice())
        .map_err(|e| format!("No se pudo derivar la clave de cifrado: {e}"))?;
    Ok(key)
}

fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = reader.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

fn io_err(context: &str, e: std::io::Error) -> String {
    format!("{context}: {e}")
}

fn encrypt_blocking(src: &Path, dst: &Path, password: &str) -> Result<(), String> {
    let mut salt = [0u8; SALT_LEN];
    let mut prefix = [0u8; PREFIX_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut prefix);

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(key.as_slice())
        .map_err(|e| format!("Clave de cifrado inválida: {e}"))?;
    let mut encryptor = EncryptorBE32::from_aead(cipher, GenericArray::from_slice(&prefix));

    let mut input = BufReader::new(File::open(src).map_err(|e| io_err("No se pudo abrir el backup", e))?);
    let mut output = BufWriter::new(
        File::create(dst).map_err(|e| io_err("No se pudo crear el archivo cifrado", e))?,
    );

    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.extend_from_slice(&salt);
    header.extend_from_slice(&prefix);
    header.extend_from_slice(&CHUNK.to_le_bytes());
    output.write_all(&header).map_err(|e| io_err("No se pudo escribir el cifrado", e))?;

    let mut buf = vec![0u8; CHUNK as usize];
    loop {
        let n = read_full(&mut input, &mut buf).map_err(|e| io_err("No se pudo leer el backup", e))?;
        if n < buf.len() {
            let block = encryptor
                .encrypt_last(&buf[..n])
                .map_err(|_| "Falló el cifrado del backup".to_string())?;
            output.write_all(&block).map_err(|e| io_err("No se pudo escribir el cifrado", e))?;
            break;
        }
        let block = encryptor
            .encrypt_next(&buf[..n])
            .map_err(|_| "Falló el cifrado del backup".to_string())?;
        output.write_all(&block).map_err(|e| io_err("No se pudo escribir el cifrado", e))?;
    }

    let file = output
        .into_inner()
        .map_err(|e| io_err("No se pudo finalizar el archivo cifrado", e.into_error()))?;
    file.sync_all().map_err(|e| io_err("No se pudo finalizar el archivo cifrado", e))?;
    Ok(())
}

/// Descifra `src`. Con `dst = None` solo comprueba que se puede descifrar entero.
fn decrypt_blocking(src: &Path, dst: Option<&Path>, password: &str) -> Result<(), String> {
    let mut input = BufReader::new(File::open(src).map_err(|e| io_err("No se pudo abrir el archivo", e))?);

    let mut header = [0u8; HEADER_LEN];
    input.read_exact(&mut header).map_err(|_| NOT_ENCRYPTED.to_string())?;
    if &header[..8] != MAGIC {
        return Err(NOT_ENCRYPTED.to_string());
    }
    if header[8] != VERSION {
        return Err("Este backup usa una versión de cifrado no compatible con esta aplicación".to_string());
    }
    let salt = &header[9..9 + SALT_LEN];
    let prefix = &header[9 + SALT_LEN..9 + SALT_LEN + PREFIX_LEN];
    let chunk = u32::from_le_bytes(header[HEADER_LEN - 4..].try_into().expect("4 bytes"));
    if chunk == 0 || chunk > MAX_CHUNK {
        return Err(NOT_ENCRYPTED.to_string());
    }

    let key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new_from_slice(key.as_slice())
        .map_err(|e| format!("Clave de cifrado inválida: {e}"))?;
    let mut decryptor = DecryptorBE32::from_aead(cipher, GenericArray::from_slice(prefix));

    let mut output = match dst {
        Some(path) => Some(BufWriter::new(
            File::create(path).map_err(|e| io_err("No se pudo crear el archivo descifrado", e))?,
        )),
        None => None,
    };

    let mut buf = vec![0u8; chunk as usize + TAG_LEN];
    let mut n =
        read_full(&mut input, &mut buf).map_err(|e| io_err("No se pudo leer el archivo", e))?;
    while n == buf.len() {
        let plain = decryptor
            .decrypt_next(&buf[..n])
            .map_err(|_| BAD_DATA.to_string())?;
        if let Some(out) = output.as_mut() {
            out.write_all(&plain)
                .map_err(|e| io_err("No se pudo escribir el archivo descifrado", e))?;
        }
        n = read_full(&mut input, &mut buf)
            .map_err(|e| io_err("No se pudo leer el archivo", e))?;
    }

    let plain = decryptor
        .decrypt_last(&buf[..n])
        .map_err(|_| BAD_DATA.to_string())?;
    if let Some(out) = output.as_mut() {
        out.write_all(&plain)
            .map_err(|e| io_err("No se pudo escribir el archivo descifrado", e))?;
    }

    if let Some(out) = output {
        let file = out
            .into_inner()
            .map_err(|e| io_err("No se pudo finalizar el archivo", e.into_error()))?;
        file.sync_all().map_err(|e| io_err("No se pudo finalizar el archivo", e))?;
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|e| format!("La tarea de cifrado se interrumpió: {e}"))?
}

pub async fn encrypt_file(src: PathBuf, dst: PathBuf, password: String) -> Result<(), String> {
    blocking(move || encrypt_blocking(&src, &dst, &password)).await
}

/// Para la restauración (fase 4).
pub async fn decrypt_file(src: PathBuf, dst: PathBuf, password: String) -> Result<(), String> {
    blocking(move || {
        let result = decrypt_blocking(&src, Some(&dst), &password);
        if result.is_err() {
            let _ = std::fs::remove_file(&dst);
        }
        result
    })
    .await
}

/// Descifra todo el archivo sin guardar nada: confirma que el backup cifrado es legible.
pub async fn verify_file(src: PathBuf, password: String) -> Result<(), String> {
    blocking(move || decrypt_blocking(&src, None, &password))
        .await
        .map_err(|e| format!("El backup cifrado no pasó la verificación: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("cfpos_test_{}_{name}", uuid::Uuid::new_v4()))
    }

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    fn roundtrip(len: usize) {
        let (plain, enc, dec) = (temp("plain"), temp("enc"), temp("dec"));
        let data = sample(len);
        std::fs::write(&plain, &data).unwrap();

        encrypt_blocking(&plain, &enc, "clave-de-prueba").unwrap();
        decrypt_blocking(&enc, None, "clave-de-prueba").unwrap();
        decrypt_blocking(&enc, Some(&dec), "clave-de-prueba").unwrap();
        assert_eq!(std::fs::read(&dec).unwrap(), data, "len = {len}");

        for path in [plain, enc, dec] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn roundtrip_edge_sizes() {
        for len in [0, 5, CHUNK as usize - 1, CHUNK as usize, CHUNK as usize + 7] {
            roundtrip(len);
        }
    }

    #[test]
    fn wrong_password_is_rejected() {
        let (plain, enc) = (temp("plain"), temp("enc"));
        std::fs::write(&plain, sample(1000)).unwrap();
        encrypt_blocking(&plain, &enc, "correcta").unwrap();
        assert_eq!(decrypt_blocking(&enc, None, "incorrecta").unwrap_err(), BAD_DATA);
        let _ = std::fs::remove_file(plain);
        let _ = std::fs::remove_file(enc);
    }

    #[test]
    fn tampering_and_truncation_are_detected() {
        let (plain, enc) = (temp("plain"), temp("enc"));
        std::fs::write(&plain, sample(CHUNK as usize * 2 + 10)).unwrap();
        encrypt_blocking(&plain, &enc, "clave").unwrap();
        let original = std::fs::read(&enc).unwrap();

        let mut flipped = original.clone();
        flipped[HEADER_LEN + 100] ^= 0xFF;
        std::fs::write(&enc, &flipped).unwrap();
        assert!(decrypt_blocking(&enc, None, "clave").is_err());

        // Cortar justo en el límite de un bloque completo.
        let cut = HEADER_LEN + CHUNK as usize + TAG_LEN;
        std::fs::write(&enc, &original[..cut]).unwrap();
        assert!(decrypt_blocking(&enc, None, "clave").is_err());

        let _ = std::fs::remove_file(plain);
        let _ = std::fs::remove_file(enc);
    }

    #[test]
    fn plain_dump_is_not_an_encrypted_backup() {
        let path = temp("plain");
        std::fs::write(&path, b"PGDMP-no-cifrado-........................").unwrap();
        assert_eq!(decrypt_blocking(&path, None, "x").unwrap_err(), NOT_ENCRYPTED);
        let _ = std::fs::remove_file(path);
    }
}
