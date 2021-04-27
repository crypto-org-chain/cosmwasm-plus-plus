import configparser
import json
import socket
import subprocess
import time


def interact(cmd, ignore_error=False, input=None, **kwargs):
    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        shell=True,
        **kwargs,
    )
    # begin = time.perf_counter()
    (stdout, _) = proc.communicate(input=input)
    # print('[%.02f] %s' % (time.perf_counter() - begin, cmd))
    if not ignore_error:
        assert proc.returncode == 0, f'{stdout.decode("utf-8")} ({cmd})'
    return stdout


def write_ini(fp, cfg):
    ini = configparser.RawConfigParser()
    for section, items in cfg.items():
        ini.add_section(section)
        sec = ini[section]
        sec.update(items)
    ini.write(fp)


def safe_cli_string(s):
    'wrap string in "", used for cli argument when contains spaces'
    s = str(s)
    if len(s.split()) > 1:
        return f"'{s}'"
    return s


def build_cli_args_safe(*args, **kwargs):
    args = [safe_cli_string(arg) for arg in args if arg]
    for k, v in kwargs.items():
        if v is None:
            continue
        args.append("--" + k.strip("_").replace("_", "-"))
        args.append(safe_cli_string(v))
    return list(map(str, args))


def build_cli_args(*args, **kwargs):
    args = [arg for arg in args if arg is not None]
    for k, v in kwargs.items():
        if v is None:
            continue
        args.append("--" + k.strip("_").replace("_", "-"))
        args.append(v)
    return list(map(str, args))


def format_doc_string(**kwargs):
    def decorator(target):
        target.__doc__ = target.__doc__.format(**kwargs)
        return target

    return decorator


class Command:
    def __init__(self, cmd, home, node, chain_id):
        self.cmd = cmd
        self.home = home
        self.node = node
        self.chain_id = chain_id

    def __call__(self, cmd, *args, stdin=None, **kwargs):
        "execute command"
        kwargs.setdefault("home", self.home)
        if cmd in ("tx", "status", "query"):
            kwargs.setdefault("node", self.node)
        if cmd == "tx":
            kwargs.setdefault("chain_id", self.chain_id)
        if cmd in ("tx", "keys"):
            kwargs.setdefault("keyring_backend", "test")
        args = " ".join(build_cli_args_safe(cmd, *args, **kwargs))
        return interact(f"{self.cmd} {args}", input=stdin)

    def wait_for_block(self, n, timeout=10):
        print("wait for block", n)
        for i in range(timeout):
            height = int(json.loads(self("status"))["SyncInfo"]["latest_block_height"])
            print("current block", height)
            if height >= n:
                break
            time.sleep(1)
        else:
            raise TimeoutError(f"wait for block {n}")


def wait_for_port(port, host="127.0.0.1", timeout=40.0):
    start_time = time.perf_counter()
    while True:
        try:
            with socket.create_connection((host, port), timeout=timeout):
                break
        except OSError as ex:
            time.sleep(0.1)
            if time.perf_counter() - start_time >= timeout:
                raise TimeoutError(
                    "Waited too long for the port {} on host {} to start accepting "
                    "connections.".format(port, host)
                ) from ex
