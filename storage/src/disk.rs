use std::{
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use uuid::Uuid;

#[derive(Debug)]
pub struct DiskStorage {
    pub base_path: PathBuf,
    pub folders: Vec<Uuid>,
}

impl DiskStorage {
    pub fn new(path: &Path) -> Result<Self, std::io::Error> {
        let folders = fs::read_dir(path);

        match folders {
            Ok(f) => {
                let folders: Vec<fs::DirEntry> = f
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|p| Path::is_dir(&p.path()))
                    .collect();

                let mut uuid_folders: Vec<_> = folders
                    .iter()
                    .map(|f| Uuid::from_str(f.file_name().to_str().unwrap()))
                    .filter_map(|uuid| uuid.ok())
                    .collect();

                uuid_folders.sort_unstable();

                dbg!(&uuid_folders);

                Ok(Self {
                    base_path: path.into(),
                    folders: uuid_folders,
                })
            }
            Err(e) => Err(e),
        }
    }

    fn get_recent_folder(&self) -> Option<&Uuid> {
        self.folders.last()
    }

    fn get_full_path(&self, folder_uid: &Uuid) -> Folder {
        let path_str = format!(
            "{}/{}",
            self.base_path.to_str().expect("no storage base path"),
            folder_uid.to_string()
        );
        let path = PathBuf::from(&path_str);

        Folder(*folder_uid, path)
    }

    fn create_new_folder(&mut self) -> std::io::Result<Folder> {
        let uid = Uuid::now_v7();
        let path = self.base_path.join(uid.to_string());

        let res = fs::create_dir(path.clone());

        match res {
            Ok(_) => Ok(Folder(uid, path)),
            Err(e) => Err(e),
        }
    }

    pub fn add_new_file(
        &mut self,
        file_contents: &[u8],
        extension: &str,
    ) -> std::io::Result<PathBuf> {
        let recent_folder = self.get_recent_folder();

        let folder = match recent_folder {
            Some(f) => self.get_full_path(f),
            None => self.create_new_folder()?,
        };
        let mut file_path = folder.path().clone();

        file_path.set_file_name(Uuid::now_v7().to_string());
        file_path.set_extension(extension);

        dbg!(&file_path.to_str());
        let file = fs::File::create(file_path.clone())?;
        let mut buf = BufWriter::new(file);

        buf.write_all(file_contents)?;

        Ok(PathBuf::from(file_path))
    }
}

pub struct Folder(Uuid, PathBuf);

impl Folder {
    pub fn uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn path(&self) -> &PathBuf {
        &self.1
    }
}

impl TryFrom<PathBuf> for Folder {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let name = value
            .file_name()
            .expect("must have filename")
            .to_str()
            .expect("should be valid str");

        let path = value.parent().expect("should not be root");

        let uuid = Uuid::parse_str(name)?;

        Ok(Folder(uuid, path.to_path_buf()))
    }
}
#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    use rand::RngCore;
    use uuid::Uuid;

    use crate::disk::Folder;

    use super::DiskStorage;

    const BASE_TMP_FOLDER: &'static str = "/tmp/pixel_tester";

    fn create_folders() -> Result<(Vec<Uuid>, Uuid), std::io::Error> {
        let mut folders = vec![
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
        ];

        let salt = Uuid::now_v7();

        for uuid in &folders {
            fs::create_dir_all(format!("{}_{}/{}", BASE_TMP_FOLDER, salt, uuid.to_string()))?
        }

        folders.sort_unstable();

        Ok((folders, salt))
    }

    fn cleanup_folders(salt: Uuid) -> std::io::Result<()> {
        let path = format!("{}_{}", BASE_TMP_FOLDER, salt);
        dbg!(&path);
        let _ = fs::remove_dir_all(path)?;

        Ok(())
    }

    #[test]
    fn test_storage_new() -> Result<(), Box<dyn Error>> {
        let (folders, salt) = create_folders()?;

        let path = PathBuf::from(format!("{}_{}", BASE_TMP_FOLDER, salt.to_string()));
        let storage = DiskStorage::new(&path.to_path_buf())?;

        assert_eq!(path.to_path_buf(), storage.base_path);
        assert_eq!(folders, storage.folders);

        let _ = cleanup_folders(salt)?;

        Ok(())
    }

    #[test]
    fn test_add_news_files_path_exist() -> Result<(), Box<dyn Error>> {
        let (_, salt) = create_folders()?;

        let path = PathBuf::from(format!("{}_{}", BASE_TMP_FOLDER, salt.to_string()));
        let mut storage = DiskStorage::new(&path.to_path_buf())?;

        let mut data = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut data);

        let res = storage.add_new_file(&data, "bin")?;

        assert!(res.exists());

        let _ = cleanup_folders(salt)?;

        Ok(())
    }

    #[test]
    fn test_add_new_file_path_dont_exists() -> Result<(), Box<dyn Error>> {
        let tmp = format!("{}_{}", BASE_TMP_FOLDER, Uuid::now_v7().to_string());
        assert!(Path::new(&tmp).exists() == false);

        let _ = fs::create_dir(tmp.clone());
        let path = PathBuf::from(tmp.clone());
        assert!(path.read_dir()?.count() == 0);

        let mut storage = DiskStorage::new(&path.to_path_buf())?;

        let mut data = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut data);

        let res = storage.add_new_file(&data, "bin")?;

        assert!(res.exists());

        let _ = fs::remove_dir_all(tmp);

        Ok(())
    }

    #[test]
    fn test_folder_pathbuf_conversion() -> Result<(), Box<dyn Error>> {
        let uid = Uuid::now_v7();
        let path = PathBuf::from("/tmp/pixel_tester/");
        let file_path = path.join(uid.to_string());

        let folder = Folder::try_from(file_path)?;

        assert_eq!(*folder.uuid(), uid, "uuid are not equal");
        assert_eq!(*folder.path(), path, "path dont match");

        Ok(())
    }
}
