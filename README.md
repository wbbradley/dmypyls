# dmypyls

/ˈdɪmpəlz/

<img src="https://github.com/user-attachments/assets/08dbd014-6ca6-47c9-b470-c45bf9d3522b" style="width: 200px">

`dmypyls` is a language server for mypy that leverages the `dmypy` daemon. `dmypyls` manages the
life-cycle of the `dmypy` daemon and provides a language server interface to it.

I'm following [this issue](https://github.com/python/mypy/issues/10463) for more info on a straight mypy
language server implementation.

## Installing dmypyls

1. Get [Rust](https://www.rust-lang.org/tools/install).
2. `cargo install dmypyls`.
3. Add `dmypyls` to your editor's language server configuration. (See below for Neovim example.)

### Running from Source

Consider using `dmypyls-debug-runner` to run from source, which is helpful for
development purposes.

## Project Configuration

In order to allow `dmypyls` to find the correct `mypy` configuration, you should place a `dmypyls.yaml` file
in the root of your project as a sibling to `mypy.ini` or `pyproject.toml`. Here are some example configurations:

If you manage your python environment with `venv` or `uv`, you'll probably want your configuration
to look like this:

```yaml
# dmypyls.yaml
dmypy_command:
  - .venv/bin/dmypy
```

If you manage your virtual environment with `pipenv`:

```yaml
# dmypyls.yaml
dmypy_command:
  - pipenv
  - run
  - dmypy
```

Or uv:

```yaml
# dmypyls.yaml
dmypy_command:
  - uv
  - --quiet
  - run
  - dmypy
```

Or pdm:

```yaml
# dmypyls.yaml
dmypy_command:
  - pdm
  - --quiet
  - run
  - dmypy
```

## User-level Configuration

You can place a `dmypyls.yaml` in your `"$HOME"/.config/dmypyls` directory to configure a fallback
behavior for all projects.

## Neovim Config

```lua
vim.api.nvim_create_autocmd({ "BufRead" }, {
  pattern = { "*.py" },
  group = vim.api.nvim_create_augroup("dmypyls-bufread", { clear = true }),
  callback = function(_)
    if vim.fn.executable("dmypyls") ~= 0 then
      -- We found an executable for dmypyls.
      vim.lsp.set_log_level(vim.log.levels.INFO)
      vim.lsp.start({
        name = "dmypyls",
        cmd = { "dmypyls", vim.api.nvim_buf_get_name(0) },
        root_dir = vim.fs.root(0, {
          ".git",
          "pyproject.toml",
          "setup.py",
          "mypy.ini"
        })
      }, { bufnr = 0, reuse_client = function(_, _) return false end })
    end
  end
})
```

### Strict Mypy config

In order for mypy usage to be helpful, I recommend ramping up the strictness of its checks. Here is
an example `mypy.ini` with relatively strict configuration.

```ini
[mypy]
strict_optional = True
show_error_codes = True
pretty = True
warn_redundant_casts = True
warn_unused_ignores = True
ignore_missing_imports = False
check_untyped_defs = True
disallow_untyped_defs = True
disallow_any_unimported = True
no_implicit_optional = True
warn_return_any = True
```
