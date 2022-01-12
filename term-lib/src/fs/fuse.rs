#![allow(unused_variables, dead_code)]
use derivative::*;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::path::Path;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::SerializationFormat;
use wasm_bus_fuse::api as backend;
use wasmer_vfs::DirEntry;
use wasmer_vfs::FileOpener;
use wasmer_vfs::FileSystem;
use wasmer_vfs::FileType;
use wasmer_vfs::FsError;
use wasmer_vfs::Metadata;
use wasmer_vfs::OpenOptions;
use wasmer_vfs::OpenOptionsConfig;
use wasmer_vfs::ReadDir;
use wasmer_vfs::VirtualFile;

use super::api::*;
use crate::api::*;
use crate::bus::AsyncWasmBusSession;
use crate::bus::WasmCallerContext;
use crate::bus::SubProcess;
use crate::stdio::Stdio;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct FuseFileSystem {
    system: System,
    #[derivative(Debug = "ignore")]
    sub: Arc<SubProcess>,
    target: String,
    #[derivative(Debug = "ignore")]
    task: AsyncWasmBusSession,
    stdio: Stdio,
    ctx: Arc<Mutex<Option<WasmCallerContext>>>,
}

impl FuseFileSystem {
    pub async fn new(
        process: Arc<SubProcess>,
        target: &str,
        mut stdio: Stdio,
    ) -> Result<FuseFileSystem, FsError> {
        let task = process
            .main
            .call::<(), _>(
                SerializationFormat::Json,
                backend::FuseMountRequest {
                    name: target.to_string(),
                },
                WasmCallerContext::default(),
            )
            .map_err(|err| {
                debug!("fuse_file_system::new() - mount call failed - {}", err);
                FsError::IOError
            })?
            .detach()
            .await
            .map_err(|err| {
                debug!(
                    "fuse_file_system::new() - detached mount call failed - {}",
                    err
                );
                FsError::IOError
            })?;
        info!("file system (target={}) opened (handle={})", target, task.id());

        let _ = stdio.stdout.flush_async().await;
        let _ = stdio.stdout.write(format!("\r").as_bytes()).await;

        let _ = task
            .call::<Result<(), backend::FsError>, _>(
                backend::FileSystemInitRequest {},
                WasmCallerContext::default()
            )
            .map_err(|err| {
                debug!("fuse_file_system::new() - mount init call failed - {}", err);
                FsError::IOError
            })?
            .join()
            .await
            .map_err(|err| {
                debug!(
                    "fuse_file_system::new() - detached mount init call failed - {}",
                    err
                );
                FsError::IOError
            })?
            .map_err(|err| {
                debug!(
                    "fuse_file_system::new() - detached mount init call failed - {}",
                    err
                );
                conv_fs_error(err)
            })?;

        let ret = FuseFileSystem {
            system: System::default(),
            sub: process,
            target: target.to_string(),
            task,
            stdio,
            ctx: Arc::new(Mutex::new(None)),
        };

        Ok(ret)
    }

    fn get_ctx(&self) -> WasmCallerContext {
        let guard = self.ctx.lock().unwrap();
        if let Some(ctx) = guard.as_ref() {
            ctx.clone()
        } else {
            WasmCallerContext::default()
        }
    }
}

impl MountedFileSystem
for FuseFileSystem
{
    fn set_ctx(&self, ctx: &WasmCallerContext) {
        let mut guard = self.ctx.lock().unwrap();
        guard.replace(ctx.clone());
    }
}

impl FileSystem
for FuseFileSystem {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        debug!("read_dir: path={}", path.display());

        let dir = self
            .task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemReadDirRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_dir(dir.map_err(conv_fs_error)?))
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        debug!("create_dir: path={}", path.display());

        let _meta = self
            .task
            .call::<Result<backend::Metadata, backend::FsError>, _>(
                backend::FileSystemCreateDirRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error)?;
        Ok(())
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        debug!("remove_dir: path={}", path.display());

        self.task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemRemoveDirRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        debug!("rename: from={}, to={}", from.display(), to.display());

        self.task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemRenameRequest {
                    from: from.to_string_lossy().to_string(),
                    to: to.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        debug!("metadata: path={}", path.display());

        let metadata = self
            .task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemReadMetadataRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_metadata(metadata.map_err(conv_fs_error)?))
    }

    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        debug!("symlink_metadata: path={}", path.display());

        let metadata = self
            .task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemReadSymlinkMetadataRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_metadata(metadata.map_err(conv_fs_error)?))
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        debug!("remove_file: path={}", path.display());

        self.task
            .call::<Result<_, backend::FsError>, _>(
                backend::FileSystemRemoveFileRequest {
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error)
    }

    fn new_open_options(&self) -> OpenOptions {
        return OpenOptions::new(Box::new(FuseFileOpener::new(self)));
    }
}

#[derive(Debug)]
pub struct FuseFileOpener {
    fs: FuseFileSystem,
}

impl FuseFileOpener {
    pub fn new(fs: &FuseFileSystem, ) -> FuseFileOpener {
        FuseFileOpener {
            fs: fs.clone()
        }
    }

    fn get_ctx(&self) -> WasmCallerContext {
        self.fs.get_ctx()
    }
}

impl FileOpener for FuseFileOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> Result<Box<dyn VirtualFile + Sync>, FsError> {
        debug!("open: path={}", path.display());

        let task = self
            .fs
            .task
            .call::<(), _>(
                backend::FileSystemOpenRequest {
                    options: backend::OpenOptions {
                        read: conf.read(),
                        write: conf.write(),
                        create_new: conf.create_new(),
                        create: conf.create(),
                        append: conf.append(),
                        truncate: conf.truncate(),
                    },
                    path: path.to_string_lossy().to_string(),
                },
                self.get_ctx()
            )
            .map_err(|err| {
                debug!("fuse_file_system::open() - open call failed - {}", err);
                FsError::IOError
            })?
            .blocking_detach()
            .map_err(|err| {
                debug!(
                    "fuse_file_system::open() - detached open call failed - {}",
                    err
                );
                FsError::IOError
            })?;

        let meta = task
            .call::<Result<_, backend::FsError>, _>(
                backend::OpenedFileMetaRequest {},
                self.get_ctx()
            )
            .map_err(|err| {
                debug!("fuse_file_system::open() - open meta call failed - {}", err);
                FsError::IOError
            })?
            .block_on()
            .map_err(|err| {
                debug!(
                    "fuse_file_system::open() - detached open meta call failed - {}",
                    err
                );
                FsError::IOError
            })?
            .map_err(|err| {
                debug!(
                    "fuse_file_system::open() - detached open meta call failed - {}",
                    err
                );
                conv_fs_error(err)
            })?;

        let io = task
            .call::<(), _>(
                backend::OpenedFileIoRequest {},
                self.get_ctx()
            )
            .map_err(|err| {
                error!("fuse_file_system::open() - open io call failed - {}", err);
                FsError::IOError
            })?
            .blocking_detach()
            .map_err(|err| {
                error!(
                    "fuse_file_system::open() - detached open io call failed - {}",
                    err
                );
                FsError::IOError
            })?;

        return Ok(Box::new(FuseVirtualFile {
            ctx: self.fs.get_ctx(),
            task,
            io,
            meta,
        }));
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FuseVirtualFile {
    ctx: WasmCallerContext,
    #[derivative(Debug = "ignore")]
    task: AsyncWasmBusSession,
    #[derivative(Debug = "ignore")]
    io: AsyncWasmBusSession,
    meta: backend::Metadata,
}

impl FuseVirtualFile {
    fn get_ctx(&self) -> WasmCallerContext {
        self.ctx.clone()
    }
}

impl Seek for FuseVirtualFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            SeekFrom::Current(a) => backend::SeekFrom::Current(a),
            SeekFrom::End(a) => backend::SeekFrom::End(a),
            SeekFrom::Start(a) => backend::SeekFrom::Start(a),
        };
        let seek = backend::FileIoSeekRequest { from: seek };

        let ret: io::Result<_> = self
            .io
            .call_with_format::<Result<_, backend::FsError>, _>(
                SerializationFormat::Bincode,
                seek,
                self.get_ctx()
            )
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }
}

impl Write for FuseVirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let ret: io::Result<_> = self
            .io
            .call_with_format::<Result<_, backend::FsError>, _>(
                SerializationFormat::Bincode,
                backend::FileIoWriteRequest { data: buf.to_vec() },
                self.get_ctx()
            )
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }

    fn flush(&mut self) -> io::Result<()> {
        let ret: io::Result<_> = self
            .io
            .call_with_format::<Result<_, backend::FsError>, _>(
                SerializationFormat::Bincode,
                backend::FileIoFlushRequest {},
                self.get_ctx()
            )
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }
}

impl Read for FuseVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: io::Result<Vec<u8>> = self
            .io
            .call_with_format::<Result<_, backend::FsError>, _>(
                SerializationFormat::Bincode,
                backend::FileIoReadRequest {
                    len: buf.len() as u64,
                },
                self.get_ctx()
            )
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        let data = data?;

        if data.len() <= 0 {
            return Ok(0usize);
        }

        let dst = &mut buf[..data.len()];
        dst.copy_from_slice(&data[..]);
        Ok(data.len())
    }
}

impl VirtualFile for FuseVirtualFile {
    fn last_accessed(&self) -> u64 {
        self.meta.accessed
    }

    fn last_modified(&self) -> u64 {
        self.meta.modified
    }

    fn created_time(&self) -> u64 {
        self.meta.created
    }

    fn size(&self) -> u64 {
        self.meta.len
    }

    fn set_len(&mut self, new_size: u64) -> Result<(), FsError> {
        let result: Result<(), FsError> = self
            .task
            .call::<Result<_, backend::FsError>, _>(
                backend::OpenedFileSetLenRequest {
                    len: new_size,
                },
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error);
        result?;

        self.meta.len = new_size;
        Ok(())
    }

    fn unlink(&mut self) -> Result<(), FsError> {
        self.task
            .call::<Result<_, backend::FsError>, _>(
                backend::OpenedFileUnlinkRequest {},
                self.get_ctx()
            )
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(conv_fs_error)
    }
}

fn conv_dir(dir: backend::Dir) -> ReadDir {
    ReadDir::new(
        dir.data
            .into_iter()
            .map(|a| conv_dir_entry(a))
            .collect::<Vec<_>>(),
    )
}

fn conv_dir_entry(entry: backend::DirEntry) -> DirEntry {
    DirEntry {
        path: Path::new(entry.path.as_str()).to_owned(),
        metadata: entry
            .metadata
            .ok_or_else(|| FsError::IOError)
            .map(|a| conv_metadata(a)),
    }
}

fn conv_metadata(metadata: backend::Metadata) -> Metadata {
    Metadata {
        ft: conv_file_type(metadata.ft),
        accessed: metadata.accessed,
        created: metadata.created,
        modified: metadata.modified,
        len: metadata.len,
    }
}

fn conv_file_type(ft: backend::FileType) -> FileType {
    FileType {
        dir: ft.dir,
        file: ft.file,
        symlink: ft.symlink,
        char_device: ft.char_device,
        block_device: ft.block_device,
        socket: ft.socket,
        fifo: ft.fifo,
    }
}

fn conv_fs_error(err: backend::FsError) -> FsError {
    match err {
        backend::FsError::BaseNotDirectory => FsError::BaseNotDirectory,
        backend::FsError::NotAFile => FsError::NotAFile,
        backend::FsError::InvalidFd => FsError::InvalidFd,
        backend::FsError::AlreadyExists => FsError::AlreadyExists,
        backend::FsError::Lock => FsError::Lock,
        backend::FsError::IOError => FsError::IOError,
        backend::FsError::AddressInUse => FsError::AddressInUse,
        backend::FsError::AddressNotAvailable => FsError::AddressNotAvailable,
        backend::FsError::BrokenPipe => FsError::BrokenPipe,
        backend::FsError::ConnectionAborted => FsError::ConnectionAborted,
        backend::FsError::ConnectionRefused => FsError::ConnectionRefused,
        backend::FsError::ConnectionReset => FsError::ConnectionReset,
        backend::FsError::Interrupted => FsError::Interrupted,
        backend::FsError::InvalidData => FsError::InvalidData,
        backend::FsError::InvalidInput => FsError::InvalidInput,
        backend::FsError::NotConnected => FsError::NotConnected,
        backend::FsError::EntityNotFound => FsError::EntityNotFound,
        backend::FsError::NoDevice => FsError::NoDevice,
        backend::FsError::PermissionDenied => FsError::PermissionDenied,
        backend::FsError::TimedOut => FsError::TimedOut,
        backend::FsError::UnexpectedEof => FsError::UnexpectedEof,
        backend::FsError::WouldBlock => FsError::WouldBlock,
        backend::FsError::WriteZero => FsError::WriteZero,
        backend::FsError::DirectoryNotEmpty => FsError::DirectoryNotEmpty,
        backend::FsError::UnknownError => FsError::UnknownError,
    }
}
