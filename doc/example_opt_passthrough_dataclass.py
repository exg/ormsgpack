import ormsgpack, dataclasses
@dataclasses.dataclass
class User:
    id: str
    name: str
    password: str

def default(obj):
    if isinstance(obj, User):
        return {"id": obj.id, "name": obj.name}
    raise TypeError

ormsgpack.packb(User("3b1", "asd", "zxc"))
ormsgpack.packb(User("3b1", "asd", "zxc"), option=ormsgpack.OPT_PASSTHROUGH_DATACLASS)
ormsgpack.packb(
    User("3b1", "asd", "zxc"),
    option=ormsgpack.OPT_PASSTHROUGH_DATACLASS,
    default=default,
)
