import code
import sys


def readline(prompt):
    try:
        line = next(sys.stdin).rstrip("\n")
    except StopIteration:
        raise EOFError()
    print(f"{prompt}{line}".rstrip(" "))
    return line


if __name__ == "__main__":
    code.interact(banner="", readfunc=readline, exitmsg="")
