fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
        res.set("ProductName", "Rust MCP Server");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("OriginalFilename", "rust-mcp-server.exe");
        res.set("InternalName", "rust-mcp-server");
        res.set("LegalCopyright", "Copyright (c) MCP Server Team");
        res.compile().unwrap();
    }
}
