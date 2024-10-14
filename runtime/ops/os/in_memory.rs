use std::collections::HashMap;

use deno_core::{OpState, op2, error::{generic_error, type_error, AnyError}};
use deno_permissions::PermissionsContainer;

use super::{
  OsError,
	NODE_ENV_VAR_ALLOWLIST,
  op_exec_path,
  op_gid,
  op_hostname,
  op_loadavg,
  op_network_interfaces,
  op_os_release,
  op_os_uptime,
  op_set_exit_code,
  op_get_exit_code,
  op_system_memory_info,
  op_uid,
  op_runtime_memory_usage,
};
use crate::worker::ExitCode;

type Env = HashMap<String, String>;

deno_core::extension!(
  deno_os,
  ops = [
    op_env,
    op_exec_path,
    op_exit,
    op_delete_env,
    op_get_env,
    op_gid,
    op_hostname,
    op_loadavg,
    op_network_interfaces,
    op_os_release,
    op_os_uptime,
    op_set_env,
    op_set_exit_code,
    op_get_exit_code,
    op_system_memory_info,
    op_uid,
    op_runtime_memory_usage,
  ],
  options = {
    exit_code: ExitCode,
    env: Env,
    exit_channel_tx: tokio::sync::watch::Sender<()>,
  },
  state = |state, options| {
    state.put::<ExitCode>(options.exit_code);
    state.put::<Env>(options.env);
    state.put::<tokio::sync::watch::Sender<()>>(options.exit_channel_tx);
  },
);

#[op2(fast, stack_trace)]
fn op_set_env(
  state: &mut OpState,
  #[string] key: &str,
  #[string] value: &str,
) -> Result<(), OsError> {
  state.borrow_mut::<PermissionsContainer>().check_env(key)?;
  if key.is_empty() {
    return Err(OsError::EnvEmptyKey);
  }
  if key.contains(&['=', '\0'] as &[char]) {
    return Err(OsError::EnvInvalidKey(key.to_string()));
  }
  if value.contains('\0') {
    return Err(OsError::EnvInvalidValue(value.to_string()));
  }
  state.borrow_mut::<Env>().insert(key.to_string(), value.to_string());
  Ok(())
}

#[op2(stack_trace)]
#[serde]
fn op_env(state: &mut OpState) -> Result<HashMap<String, String>, AnyError> {
  state.borrow_mut::<PermissionsContainer>().check_env_all()?;
  Ok(state.borrow::<Env>().clone())
}

#[op2(stack_trace)]
#[string]
fn op_get_env(
  state: &mut OpState,
  #[string] key: String,
) -> Result<Option<String>, AnyError> {
  let skip_permission_check = NODE_ENV_VAR_ALLOWLIST.contains(&key);

  if !skip_permission_check {
    state.borrow_mut::<PermissionsContainer>().check_env(&key)?;
  }

  if key.is_empty() {
    return Err(type_error("Key is an empty string."));
  }

  if key.contains(&['=', '\0'] as &[char]) {
    return Err(type_error(format!(
      "Key contains invalid characters: {key:?}"
    )));
  }

  Ok(state.borrow::<Env>().get(&key).cloned())
}

#[op2(fast, stack_trace)]
fn op_delete_env(
  state: &mut OpState,
  #[string] key: String,
) -> Result<(), AnyError> {
  state.borrow_mut::<PermissionsContainer>().check_env(&key)?;
  if key.is_empty() || key.contains(&['=', '\0'] as &[char]) {
    return Err(type_error("Key contains invalid characters."));
  }
  state.borrow_mut::<Env>().remove(&key);
  Ok(())
}

#[op2(fast, stack_trace)]
fn op_exit(state: &mut OpState) -> Result<(), AnyError> {
  if state.borrow::<tokio::sync::watch::Sender<()>>().send(()).is_err() {
    return Err(generic_error("Failed to send exit signal."));
  }
  Ok(())
}
