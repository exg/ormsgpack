import dataclasses, ormsgpack, typing
@dataclasses.dataclass
class Member:
    id: int
    active: bool = dataclasses.field(default=False)

@dataclasses.dataclass
class Object:
    id: int
    name: str
    members: typing.List[Member]

ormsgpack.packb(Object(1, "a", [Member(1, True), Member(2)]))
