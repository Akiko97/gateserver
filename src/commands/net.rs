use crate::ServerContext;
use super::ArgSlice;

pub async fn reconnect(
    args: ArgSlice<'_>,
    state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    //
    Ok(String::from(""))
}
