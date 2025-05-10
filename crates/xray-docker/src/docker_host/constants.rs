#[cfg(windows)]
pub const DEFAULT_DOCKER_HOST: &str = "npipe:////.pipe/docker_engine";

#[cfg(unix)]
pub const DEFAULT_DOCKER_HOST: &str = "unix:///var/run/docker.sock";

pub const DOCKER_HOST_ENV_VAR: &str = "DOCKER_HOST";

pub const DOCKER_CONFIG_DIR_ENV_VAR: &str = "DOCKER_CONFIG";

pub const DOCKER_DEFAULT_CONFIG_DIR: &str = ".docker";

pub const DOCKER_CONTEXTS_METADATA_DIR: &str = "contexts/meta";
