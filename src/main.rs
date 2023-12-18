use libc::{ENOENT, EROFS};
use std::{
    collections::HashMap,
    ffi::OsStr,
    time::{Duration, UNIX_EPOCH},
};

use fuser::{
    consts::FOPEN_DIRECT_IO, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

// const HELLO_TXT_ATTR: FileAttr = FileAttr {
//     ino: 2,
//     size: 13,
//     blocks: 1,
//     atime: UNIX_EPOCH, // 1970-01-01 00:00:00
//     mtime: UNIX_EPOCH,
//     ctime: UNIX_EPOCH,
//     crtime: UNIX_EPOCH,
//     kind: FileType::RegularFile,
//     perm: 0o644,
//     nlink: 1,
//     uid: 501,
//     gid: 20,
//     rdev: 0,
//     flags: 0,
//     blksize: 512,
// };

struct FuseFsFile {
    name: String,
    attr: FileAttr,
    data: Vec<u8>,
}

struct FuseFS<'a> {
    files: &'a mut Vec<Box<FuseFsFile>>,
    ino_count: &'a mut u64,
    fh_count: &'a mut u64,
    ino_to_names: &'a mut HashMap<u64, String>,
    ino_to_fh: &'a mut HashMap<u64, u64>,
}

impl<'a> Filesystem for FuseFS<'a> {
    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        dbg!("CREATE", name, mode, umask, flags);
        if parent == 1 {
            *self.ino_count += 1;
            *self.fh_count += 1;
            let ino = *self.ino_count;
            let fh = *self.fh_count;
            let ttl = &Duration::MAX;
            let flags = flags as u32;
            let attr = FileAttr {
                ino,
                size: 0,
                blocks: 1,
                atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: _req.uid(),
                gid: _req.gid(),
                rdev: 0,
                flags,
                blksize: 512,
            };
            let name = String::from(name.to_str().unwrap());

            self.ino_to_names.insert(ino, name.clone());
            self.ino_to_fh.insert(ino, fh);
            self.files.push(Box::new(FuseFsFile {
                name,
                attr,
                data: vec![],
            }));

            reply.created(ttl, &attr, ino, fh, flags)
        } else {
            reply.error(EROFS)
        }
    }

    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        dbg!("OPEN", _ino, _flags);

        let fh = self.ino_to_fh.get(&_ino).unwrap();

        reply.opened(*fh, FOPEN_DIRECT_IO)
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        _flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        dbg!("WRITE", ino, fh, offset, write_flags);
        let file = self.files.into_iter().find(|f| f.attr.ino == ino).unwrap();

        file.data = data.to_vec();

        file.attr.size = data.len() as u64;

        reply.written(data.len() as u32)
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        dbg!("LOOKUP", parent, name);

        if let Some(file_name) = name.to_str() {
            let f = self.files.iter().find(|f| f.name == file_name);
            match f {
                Some(f) => reply.entry(&TTL, &f.attr, 0),
                None => reply.error(ENOENT),
            };
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        dbg!("GETATTR", ino);
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            i => {
                let f = self.files.iter().find(|f| f.attr.ino == i);
                match f {
                    Some(f) => reply.attr(&TTL, &f.attr),
                    None => reply.error(ENOENT),
                }
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        dbg!("READ", ino, _fh, offset);
        let f = self.files.iter().find(|f| f.attr.ino == ino);
        match f {
            Some(f) => reply.data(&f.data[offset as usize..]),
            None => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        dbg!("READ", ino, _fh, offset);
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries = self
            .files
            .into_iter()
            .map(|f| return (f.attr.ino, FileType::RegularFile, f.name.clone()))
            .collect::<Vec<(u64, FileType, String)>>();

        entries.push((1, FileType::Directory, ".".to_owned()));
        entries.push((1, FileType::Directory, "..".to_owned()));

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

fn main() {
    // TODO Make this parametrized
    let mountpoint = "/tmp/fusefs";
    let fs_name = String::from("fusefs");
    let options = vec![
        MountOption::RW,
        MountOption::AutoUnmount,
        MountOption::AllowOther,
        MountOption::FSName(fs_name),
    ];

    let mut files = vec![];
    let mut ino_count = 1;
    let mut fh_count = 1;
    let mut ino_to_names: HashMap<u64, String> = HashMap::new();
    let mut fh_to_ino: HashMap<u64, u64> = HashMap::new();
    let fs = FuseFS {
        files: &mut files,
        ino_count: &mut ino_count,
        fh_count: &mut fh_count,
        ino_to_fh: &mut fh_to_ino,
        ino_to_names: &mut ino_to_names,
    };

    fuser::mount2(fs, mountpoint, &options).unwrap();
}
