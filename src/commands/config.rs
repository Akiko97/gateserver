use crate::config::SERVER_CONFIG;
use crate::ServerContext;
use super::ArgSlice;

pub async fn timeout(
    args: ArgSlice<'_>,
    state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    //
    Ok(String::from(""))
}

pub async fn save(
    args: ArgSlice<'_>,
    state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    //
    Ok(String::from(""))
}
