local rust mcp server designed to easily implement new tools.

create a new tool in /tools and add the router block/line in main.rs and mod.rs  using the boilerplate_example.rs to structure the tool.

in lmstudio:
mcp.json:
{
  "mcpServers": {
    "rust-mcp-server": {
      "command": "C:\\Users\\acana\\source\\repos\\mcp\\target\\debug\\mcp.exe",
      "args": []
    }
  }
}

sloppy/simple approach but seems to work | replacing my command path with yours, needs to be double slashed | .exe doesn't need to be running, just needs to exist at the path.
tool args defined in the code.
