# dmypyls

`dmypyls` is a language server for the mypy type checker's `dmypy` daemon. `dmypyls` manages the
lifecycle of the `dmypy` daemon and provides a language server interface to it.

I should mention that there is plenty of prior art in this space. See
[sileht/dmypy-ls](https://github.com/sileht/dmypy-ls) for one example. Also, the mypy folks seem to
want to convert `dmypy` to a language server. See [this issue](https://github.com/python/mypy/issues/10463).

## Installing dmypyls

Assuming you have a recent version of the rust toolchain installed, you should be able to `cargo
install dmypyls`. For now, you'll need to ensure that `mypy` is in your path while running `dmypyls`.

### Running from Source

Consider using `dmypyls-debug-runner` to run from source, which is helpful for
development purposes.

## Project Configuration

In order to allow dmypyls to find the correct `mypy` configuration, you should place a `dmypyls.yaml` file
in the root of your project. Here is some example configurations:

If you manage your virtual environment manually with `venv` or `uv`:

```yaml
# dmypyls.yaml
python_execution_path: .venv/bin/python
```

If you manage your virtual environment with `poetry`:

```yaml
# dmypyls.yaml
python_execution_path: poetry
```

Or pipenv:

```yaml
# dmypyls.yaml
python_execution_path: pipenv
```

Or pdm:

```yaml
# dmypyls.yaml
python_execution_path: pdm
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
