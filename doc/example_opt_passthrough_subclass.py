import ormsgpack
class Secret(str):
    pass

def default(obj):
    if isinstance(obj, Secret):
        return "******"
    raise TypeError

ormsgpack.packb(Secret("zxc"))
ormsgpack.packb(Secret("zxc"), option=ormsgpack.OPT_PASSTHROUGH_SUBCLASS)
ormsgpack.packb(Secret("zxc"), option=ormsgpack.OPT_PASSTHROUGH_SUBCLASS, default=default)
