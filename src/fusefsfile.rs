use std::time::{SystemTime, UNIX_EPOCH};

use fuser::{consts::FOPEN_DIRECT_IO, FileAttr, FileType};

const BLOCK_SIZE: u32 = 512;

pub struct FuseFsFile {
    pub name: String,
    pub attr: FileAttr,
    pub file_type: FileType,
    pub data: Vec<u8>,
}

impl FuseFsFile {
    pub fn create_file_attr(ino: u64, uid: u32, gid: u32, flags: u32) -> FileAttr {
        let created_time = SystemTime::now();

        FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: created_time,
            ctime: created_time,
            crtime: created_time,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 2,
            uid,
            gid,
            rdev: 0,
            flags,
            blksize: BLOCK_SIZE,
        }
    }

    pub fn create_dir_attr(ino: u64, uid: u32, gid: u32) -> FileAttr {
        let created_time = SystemTime::now();

        FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: created_time,
            ctime: created_time,
            crtime: created_time,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid,
            gid,
            rdev: 0,
            flags: 0,
            blksize: BLOCK_SIZE,
        }
    }
}
