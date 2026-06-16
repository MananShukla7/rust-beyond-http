use libunftp::auth::{AuthenticationError, Authenticator, Credentials, UserDetail};
use std::sync::Arc;
use tracing::info;
use unftp_sbe_fs::{Filesystem, Meta};
use unftp_sbe_restrict::{RestrictingVfs, UserWithPermissions, VfsOperations};

#[derive(Debug, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub permissions: VfsOperations,
}

impl UserDetail for User {
    fn account_enabled(&self) -> bool { true }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User(username: {})", self.username)
    }
}

impl UserWithPermissions for User {
    fn permissions(&self) -> VfsOperations {
        self.permissions
    }
}

/// Simple username/password authenticator
#[derive(Debug)]
struct PasswordAuth {
    username: String,
    password: String,
}

#[async_trait::async_trait]
impl Authenticator<User> for PasswordAuth {
    async fn authenticate(
        &self,
        username: &str,
        creds: &Credentials,
    ) -> Result<User, AuthenticationError> {
        if username != self.username {
            return Err(AuthenticationError::BadUser);
        }
        match &creds.password {
            Some(pw) if pw.as_str() == self.password => {
                // Grant upload-only permissions: allow GET, PUT, LIST — block DEL, REN, MKD, RMD
                Ok(User {
                    username: username.to_string(),
                    permissions: VfsOperations::all()
                        - VfsOperations::DEL
                        - VfsOperations::RENAME
                        - VfsOperations::MK_DIR
                        - VfsOperations::RM_DIR,
                })
            }
            _ => Err(AuthenticationError::BadPassword),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let root = std::env::var("FTP_ROOT").unwrap_or_else(|_| "/tmp/ftp-uploads".to_string());
    std::fs::create_dir_all(&root)?;
    info!("FTP root: {}", root);

    let auth = Arc::new(PasswordAuth {
        username: std::env::var("FTP_USER").unwrap_or_else(|_| "uploader".to_string()),
        password: std::env::var("FTP_PASS").unwrap_or_else(|_| "secret123".to_string()),
    });

    let root_clone = root.clone();
    let backend = Box::new(move || {
        RestrictingVfs::<Filesystem, User, Meta>::new(Filesystem::new(root_clone.clone()))
    });

    // Build server with filesystem backend wrapped by the restricting VFS
    let server = libunftp::ServerBuilder::<RestrictingVfs<Filesystem, User, Meta>, User>::with_authenticator(backend, auth)
        .build()?;

    let addr = "0.0.0.0:2121";
    info!("FTP server listening on {}", addr);
    server.listen(addr).await?;

    Ok(())
}
