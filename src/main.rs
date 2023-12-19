mod fusefs;
mod fusefsfile;

use std::io;

use fusefs::FuseFS;

fn main() -> io::Result<()> {
    let mountpoint = String::from("/tmp/fusefs");
    let fs_name = String::from("fusefs");

    let fs = FuseFS::init(mountpoint, fs_name);
    fs.mount_rw_create()
}
