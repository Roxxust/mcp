# local rust mcp server designed to easily implement new tools.

create a new tool in /tools and add the router block/line in main.rs and mod.rs  using the boilerplate_example.rs to structure the tool.<br/>
tool args are defined in the code.<br/>
sloppy/simple approach but seems to work..<br/>

*replace **insertuserhere** in the path with yours in the [**mcp.json**](https://github.com/Roxxust/mcp/blob/main/mcp.json%20for%20lm%20studio) for lm studio(needs to be double slashed)* <br/>

.exe doesn't need to be running, just needs to exist at the path.<br/>

server address in .env so it doesn't need to be re-compiled to change it.<br/>

## current tools:
#### **boilerplate_example.rs**:
 an example of the boilerplate for the tools main.rs expects. simple echo back if tool used.<br/>
#### **get_time.rs**:
 gets the user's time in local format, 12hr by default or the supported formats if requested.<br/>
#### **query_rustdocs.rs**:
 attempts to parse current rust crate version for the crate the llm intends to use and pull some hopefully updated documentation. (experimental, works better if you tell it which crates you want info on)<br/>
