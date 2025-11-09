fn main() -> anyhow::Result<()> {
    tokio::runtime::Runtime::new()?.block_on(control_plane::start())
}
