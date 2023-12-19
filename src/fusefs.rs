use crate::fusefsfile::FuseFsFile;
use fuser::{
    consts::FOPEN_DIRECT_IO, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use libc::{EBADF, ENOENT, EROFS, O_APPEND, O_RDWR, O_TRUNC, O_WRONLY};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{self, ErrorKind},
    time::Duration,
};

pub struct FuseFS {
    mountpoint: String,
    fs_name: String,
    files: Vec<Box<FuseFsFile>>,
    ino_count: u64,
    fh_count: u64,
    ino_to_names: HashMap<u64, String>,
    ino_to_fh: HashMap<u64, u64>,
}

impl FuseFS {
    pub fn init(mountpoint: String, fs_name: String) -> Self {
        let files = vec![];
        let ino_count = 1;
        let fh_count = 1;
        let ino_to_names: HashMap<u64, String> = HashMap::new();
        let ino_to_fh: HashMap<u64, u64> = HashMap::new();

        FuseFS {
            mountpoint,
            fs_name,
            files,
            ino_count,
            fh_count,
            ino_to_fh,
            ino_to_names,
        }
    }

    pub fn mount_rw(self) -> io::Result<()> {
        let fs_name = self.fs_name.clone();
        let mountpoint = self.mountpoint.clone();

        let options = vec![
            MountOption::RW,
            MountOption::AutoUnmount,
            MountOption::AllowOther,
            MountOption::FSName(fs_name),
        ];

        fuser::mount2(self, mountpoint, &options)
    }

    pub fn mount_rw_create(self) -> io::Result<()> {
        let mountpoint = self.mountpoint.clone();

        FuseFS::create_dir_if_not_exist(&mountpoint);

        self.mount_rw()
    }

    fn create_dir_if_not_exist(path: &str) {
        let dir_exists = fs::read_dir(&path);
        match dir_exists {
            Ok(_) => {}
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    println!("Attempting to create dir {}", &path);

                    fs::create_dir(path).unwrap();
                }
            }
        }
    }
}

const TTL: Duration = Duration::from_secs(1); // 1 second

impl<'a> Filesystem for FuseFS {
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

        self.ino_count += 1;
        self.fh_count += 1;
        let ino = self.ino_count;
        let fh = self.fh_count;
        let ttl = &Duration::MAX;
        let flags = flags as u32;

        let attr = FuseFsFile::create_file_attr(ino, _req.uid(), _req.gid(), flags);
        let name = String::from(name.to_str().unwrap());

        self.ino_to_names.insert(ino, name.clone());
        self.ino_to_fh.insert(ino, fh);
        self.files.push(Box::new(FuseFsFile {
            name,
            attr,
            file_type: FileType::RegularFile,
            data: vec![],
        }));

        reply.created(ttl, &attr, ino, fh, flags)
    }

    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        dbg!("OPEN", _ino, _flags);

        let fh = self.ino_to_fh.get(&_ino);

        match fh {
            Some(fh) => reply.opened(*fh, FOPEN_DIRECT_IO),
            None => {
                self.fh_count += 1;
                self.ino_to_fh.insert(_ino, self.fh_count);
                reply.opened(self.fh_count, FOPEN_DIRECT_IO);
            }
        }
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        dbg!("FLUSH", ino, fh);

        self.ino_to_fh.remove(&ino);

        reply.ok()
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
        let stringdata = String::from_utf8_lossy(data);
        dbg!(stringdata);

        let file = (&mut self.files)
            .into_iter()
            .find(|f| f.attr.ino == ino)
            .unwrap();

        file.data.append(&mut data.to_vec());

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
            1 => {
                let attr = FuseFsFile::create_dir_attr(ino, _req.uid(), _req.gid());
                reply.attr(&TTL, &attr);
            }
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

        let mut entries = (&mut self.files)
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
