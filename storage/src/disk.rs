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
    #[tracing::instrument]
    pub fn new(path_str: &'a str) -> Result<Self> {
        let path = Path::new(path_str);
        tracing::info!("Initializing disk storage at: {:?}", path.to_str());

        if !path.exists() {
            tracing::info!("Path '{}' not found. Creating entire path.", path_str);
            fs::create_dir_all(path)?
        }

        Ok(Self { base_path: path })
    }

    #[tracing::instrument]
    pub fn from_path(path: &'a Path) -> Result<Self> {
        tracing::info!("Initializing disk storage at: {:?}", path.to_str());

        if !path.exists() {
            tracing::info!("Path '{:?}' not found. Creating entire path.", path);
            fs::create_dir_all(path)?
        }

        Ok(Self { base_path: path })
    }

    #[tracing::instrument(skip(data))]
    pub fn add_new_file(&self, file: File, data: &[u8]) -> std::io::Result<PathBuf> {
        let file_path = self.base_path.join(file.file_name());

        let file_handler = fs::File::create(file_path.clone())?;
        let mut buf = BufWriter::new(file_handler);

        buf.write_all(data)?;
        tracing::debug!("created new file at {:?}", file_path.to_str());

        Ok(file_path.to_path_buf())
    }

    #[tracing::instrument]
    pub fn delete_file(&self, file: File) -> std::io::Result<()> {
        let file_path = self.base_path.join(file.file_name());

        fs::remove_file(file_path)
    }
}

#[derive(Debug)]
pub struct File<'a>(&'a str, &'a str);

impl<'a> File<'a> {
    pub fn new(name: &'a str, extenstion: &'a str) -> Self {
        Self(name, extenstion)
    }

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

    const BASE_TMP_FOLDER: &str = "/tmp/pixel_tester";

    fn create_random_folder() -> String {
        format!("{}_{}", BASE_TMP_FOLDER, Uuid::now_v7().as_simple())
    }

    #[test]
    fn test_storage_new_folder_dont_exist() -> io::Result<()> {
        let folder = create_random_folder();

        let _ = DiskStorage::new(&folder)?;

        assert!(Path::new(&folder).exists());

        fs::remove_dir(folder)?;

        Ok(())
    }

    #[test]
    fn test_storage_new_folder_exists() -> io::Result<()> {
        let folder = create_random_folder();
        fs::create_dir_all(&folder)?;

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

    #[test]
    fn test_delete_file() -> io::Result<()> {
        let folder = create_random_folder();

        let storage = DiskStorage::new(&folder)?;

        let file = super::File("empty", "jpg");

        let mut data = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut data);

        let path = storage.add_new_file(file, &data)?;

        let file = super::File("empty", "jpg");
        storage.delete_file(file)?;

        assert!(!path.exists());

        fs::remove_dir(folder)?;

        Ok(())
    }
}
