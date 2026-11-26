pub struct Unpacked {
    pub dir: std::path::PathBuf,
}

impl Drop for Unpacked {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub const UNPACK_HEAD: &str = "eravon-unpack-";

pub fn unpack(file: &std::path::Path) -> Result<Unpacked, crate::container::Bad> {
    let mut n = [0u8; 8];
    eravon_os::fill_random(&mut n).map_err(|e| crate::container::Bad::Io(format!("read {}: {e}", eravon_os::ENTROPY_SOURCE)))?;
    let dir = std::env::temp_dir().join(format!("{UNPACK_HEAD}{}", n.iter().map(|b| format!("{b:02x}")).collect::<String>()));
    crate::container::unpack_to(file, &dir)?;
    Ok(Unpacked { dir })
}
