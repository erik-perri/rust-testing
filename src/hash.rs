use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};

pub fn random_sha1_to_string() -> String {
    let mut rng = thread_rng();
    let mut bytes: [u8; 64] = [0; 64];
    rng.fill(&mut bytes[..]);

    let mut hasher = Sha1::new();
    hasher.update(bytes);

    let sha1 = hasher.finalize();

    format!("{:x}", sha1)
}
