import dataclasses
import datetime
import enum
import random
import unicodedata
import uuid
from collections.abc import Callable

import msgpack
import numpy
from pydantic import BaseModel

import ormsgpack


@dataclasses.dataclass(frozen=True)
class Experiment:
    name: str
    data: object
    unpack: bool


@dataclasses.dataclass
class Group:
    name: str
    uid: uuid.UUID = dataclasses.field(default_factory=uuid.uuid4)


class UserType(enum.Enum):
    Admin = 1
    User = 2
    System = 3


@dataclasses.dataclass
class User:
    active: bool
    ctime: datetime.datetime
    display_name: str
    groups: list[Group]
    name: str
    score: float
    type: UserType
    uid: uuid.UUID = dataclasses.field(default_factory=uuid.uuid4)


class GroupModel(BaseModel):
    name: str
    uid: uuid.UUID


class UserModel(BaseModel):
    active: bool
    ctime: datetime.datetime
    display_name: str
    groups: list[GroupModel]
    name: str
    score: float
    type: UserType
    uid: uuid.UUID


class Generator:
    @staticmethod
    def letters(a: int, b: int) -> list[str]:
        return [
            chr(cp) for cp in range(a, b) if unicodedata.category(chr(cp))[0] in "L"
        ]

    def __init__(self) -> None:
        self.alphabets = [
            self.letters(0, 0x80),
            self.letters(0x3040, 0x30A0),
        ]
        self.groups = [
            Group(name="daemon"),
            Group(name="mail"),
            Group(name="wheel"),
        ]
        self.min_time = int(
            datetime.datetime(1970, 1, 1, tzinfo=datetime.timezone.utc).timestamp()
        )
        self.max_time = int(
            datetime.datetime(2514, 1, 1, tzinfo=datetime.timezone.utc).timestamp()
        )

    def string(self) -> str:
        length = random.randint(1, 64)
        alphabet = random.choice(self.alphabets)
        return "".join(random.choice(alphabet) for _ in range(length))

    def datetime(self) -> datetime.datetime:
        timestamp = random.randint(self.min_time, self.max_time)
        return datetime.datetime.fromtimestamp(timestamp, tz=datetime.timezone.utc)

    def user(self) -> User:
        name = self.string()
        return User(
            active=random.choice((True, False)),
            ctime=self.datetime(),
            display_name=self.string(),
            groups=[
                Group(name=name),
                *random.sample(self.groups, k=random.randint(1, len(self.groups))),
            ],
            name=name,
            score=random.random(),
            type=random.choice((UserType.Admin, UserType.User, UserType.System)),
        )

    def experiments(self) -> list[Experiment]:
        users = [self.user() for _ in range(100)]
        users_as_dict = [dataclasses.asdict(v) for v in users]
        numpy_rng = numpy.random.default_rng()
        return [
            Experiment(
                name="serialization",
                data=users_as_dict,
                unpack=False,
            ),
            Experiment(
                name="dataclass serialization",
                data=users,
                unpack=False,
            ),
            Experiment(
                name="pydantic serialization",
                data=[UserModel(**v) for v in users_as_dict],
                unpack=False,
            ),
            Experiment(
                name="deserialization",
                data=ormsgpack.packb(users),
                unpack=True,
            ),
            Experiment(
                name="numpy int32 serialization",
                data=numpy_rng.integers(2**31, size=(100000, 100), dtype=numpy.int32),
                unpack=False,
            ),
            Experiment(
                name="numpy float64 serialization",
                data=numpy_rng.random(size=(50000, 100)),
                unpack=False,
            ),
            Experiment(
                name="numpy bool serialization",
                data=numpy_rng.choice((True, False), size=(100000, 200)),
                unpack=False,
            ),
        ]


@dataclasses.dataclass(frozen=True)
class Library:
    name: str
    packb: Callable[[object], bytes]
    unpackb: Callable[[bytes], object]


def default(obj: object) -> object:
    if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
        return dataclasses.asdict(obj)
    if isinstance(obj, BaseModel):
        return obj.model_dump()
    if isinstance(obj, numpy.ndarray):
        return obj.tolist()
    return str(obj)


LIBRARIES = (
    Library(
        name="msgpack",
        packb=lambda x: msgpack.packb(x, default=default),
        unpackb=msgpack.unpackb,
    ),
    Library(
        name="ormsgpack",
        packb=lambda x: ormsgpack.packb(
            x,
            option=ormsgpack.OPT_SERIALIZE_NUMPY | ormsgpack.OPT_SERIALIZE_PYDANTIC,
        ),
        unpackb=ormsgpack.unpackb,
    ),
)
