## ``lupat``
> Lua's patterns in Rust

Adds [lua patterns](https://www.lua.org/pil/20.2.html) to be used in your Rust libraries.  
These are essentially simpler albeit slightly more limited Regex patterns, just missing stuff like ``|`` and ``{2}``.  
This has no dependencies, so could be preferable instead of using the massive ``regex`` crate.

Forked from [lua-patterns](https://github.com/stevedonovan/lua-patterns).
Plan is to rewrite it completely to no longer need unsafe code.  
Already stripped out all of the possible panics.
