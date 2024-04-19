use fs2::FileExt;
use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};
use std::fs::{File, OpenOptions};

pub fn calculate_xor_distance(node_id: &str, target_id: &str) -> Result<u32, String> {
    let node_id_bytes =
        sha1_to_bytes(node_id).map_err(|_| format!("Invalid node ID: {}", node_id))?;
    let target_id_bytes =
        sha1_to_bytes(target_id).map_err(|_| format!("Invalid target ID: {}", target_id))?;

    Ok(distance_to_node(&node_id_bytes, &target_id_bytes))
}

fn distance_to_node(node_id_a: &[u8; 20], node_id_b: &[u8; 20]) -> u32 {
    let mut distance = 0;

    for i in 0..20 {
        let xor = node_id_a[i] ^ node_id_b[i];
        if xor != 0 {
            distance = 8 * (19 - i) + xor.leading_zeros() as usize;
            break;
        }
    }

    distance as u32
}

pub fn lock_file(path: &str) -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(path)
        .map_err(|error| format!("Failed to open file \"{}\": {}", path, error))?;

    file.try_lock_exclusive().map_err(|error| {
        format!(
            "Failed to lock file \"{}\" for writing: {}",
            path,
            error.to_string()
        )
    })?;

    Ok(file)
}

pub fn random_sha1_to_string() -> String {
    let mut rng = thread_rng();
    let mut bytes: [u8; 64] = [0; 64];
    rng.fill(&mut bytes[..]);

    let mut hasher = Sha1::new();
    hasher.update(bytes);

    let sha1 = hasher.finalize();

    format!("{:x}", sha1)
}

pub fn sha1_to_bytes(sha1: &str) -> Result<[u8; 20], std::num::ParseIntError> {
    let mut bytes: [u8; 20] = [0; 20];

    for i in 0..20 {
        bytes[i] = u8::from_str_radix(&sha1[i * 2..i * 2 + 2], 16)?;
    }

    Ok(bytes)
}
