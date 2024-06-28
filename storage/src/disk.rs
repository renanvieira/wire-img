use std::{
    fs,
    io::{BufWriter, Result, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct DiskStorage<'a> {
    pub base_path: &'a Path,
}

impl<'a> DiskStorage<'a> {
    pub fn new(path_str: &'a str) -> Result<Self> {
        let path = Path::new(path_str);
        tracing::info!("Initializing disk storage at: {:?}", path.to_str());

        if path.exists() == false {
            tracing::info!("Path '{}' not found. Creating entire path.", path_str);
            fs::create_dir_all(&path)?
        }

        Ok(Self { base_path: path })
    }

    pub fn add_new_file(&self, file: File, data: &[u8]) -> std::io::Result<PathBuf> {
        let file_path = self.base_path.join(file.file_name());

        let file_handler = fs::File::create(file_path.clone())?;
        let mut buf = BufWriter::new(file_handler);

        buf.write_all(data)?;
        tracing::debug!("created new file at {:?}", file_path.to_str());

        Ok(file_path.to_path_buf())
    }
}

#[derive(Debug)]
pub struct File<'a>(&'a str, &'a str);

impl<'a> File<'a> {
    pub fn name(&self) -> &str {
        self.0
    }

    pub fn extension(&self) -> &str {
        self.1
    }

    pub fn file_name(&self) -> String {
        format!("{}.{}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io, path::Path};

    use rand::RngCore;
    use uuid::Uuid;

    use super::DiskStorage;

    const BASE_TMP_FOLDER: &'static str = "/tmp/pixel_tester";

    fn create_random_folder() -> String {
        format!(
            "{}_{}",
            BASE_TMP_FOLDER,
            Uuid::now_v7().as_simple().to_string()
        )
    }

    #[test]
    fn test_storage_new_folder_dont_exist() -> io::Result<()> {
        let folder = create_random_folder();

        let _ = DiskStorage::new(&folder)?;

        assert!(Path::new(&folder).exists());

        let _ = fs::remove_dir(folder)?;

        Ok(())
    }

    #[test]
    fn test_storage_new_folder_exists() -> io::Result<()> {
        let folder = create_random_folder();
        let _ = fs::create_dir_all(&folder)?;

        let _ = DiskStorage::new(&folder)?;

        assert!(Path::new(&folder).exists());

        fs::remove_dir(folder)?;

        Ok(())
    }

    #[test]
    fn test_add_new_file() -> io::Result<()> {
        let folder = create_random_folder();

        let storage = DiskStorage::new(&folder)?;

        let file = super::File("empty", "jpg");
        let filename = file.file_name();

        let mut data = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut data);

        let path = storage.add_new_file(file, &data)?;

        assert_eq!(
            format!("{}/{}", folder, filename),
            path.to_str().unwrap_or_default()
        );

        fs::remove_file(path)?;
        fs::remove_dir(folder)?;

        Ok(())
    }
}
