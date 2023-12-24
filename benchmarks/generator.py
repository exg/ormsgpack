import dataclasses
import datetime
import enum
import uuid
from collections.abc import Callable

import faker
import msgpack
import numpy
from pydantic import BaseModel

import ormsgpack


@dataclasses.dataclass(frozen=True)
class Experiment:
    name: str
    data: object


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
    groups: list[GroupModel]
    name: str
    score: float
    type: UserType
    uid: uuid.UUID


class Generator:
    def __init__(self) -> None:
        self.faker = faker.Faker(["en_US", "ja_JP"])
        self.rng = numpy.random.default_rng()
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

    def boolean(self) -> bool:
        return bool(self.rng.choice((True, False)))

    def datetime(self) -> datetime.datetime:
        timestamp = self.rng.integers(self.min_time, self.max_time)
        return datetime.datetime.fromtimestamp(timestamp, tz=datetime.timezone.utc)

    def uuid4(self) -> uuid.UUID:
        return uuid.UUID(bytes=self.rng.bytes(16), version=4)

    def user(self) -> User:
        name = self.faker.name()
        return User(
            active=self.boolean(),
            ctime=self.datetime(),
            groups=[
                Group(name=name),
                *self.rng.choice(
                    self.groups,
                    size=self.rng.integers(1, len(self.groups)),
                    replace=False,
                ),
            ],
            name=name,
            score=self.rng.random(),
            type=self.rng.choice(list(UserType)),
        )

    def experiments(self) -> list[Experiment]:
        users = [self.user() for _ in range(10000)]
        users_as_dicts = [dataclasses.asdict(user) for user in users]
        users_as_models = [UserModel(**user) for user in users_as_dicts]
        bool_array = self.rng.choice((True, False), size=(10000, 10))
        float_array = self.rng.random(size=(10000, 10))
        int_array = self.rng.integers(low=-(2**31), high=2**31, size=(10000, 10))
        return [
            Experiment(
                name="dict",
                data=users_as_dicts,
            ),
            Experiment(
                name="dataclass",
                data=users,
            ),
            Experiment(
                name="pydantic",
                data=users_as_models,
            ),
            Experiment(
                name="bool",
                data=bool_array.tolist(),
            ),
            Experiment(
                name="float",
                data=float_array.tolist(),
            ),
            Experiment(
                name="int",
                data=int_array.tolist(),
            ),
            Experiment(
                name="numpy.bool",
                data=bool_array,
            ),
            Experiment(
                name="numpy.float",
                data=float_array,
            ),
            Experiment(
                name="numpy.int",
                data=int_array,
            ),
            Experiment(
                name="datetime",
                data=[self.datetime() for _ in range(100000)],
            ),
            Experiment(
                name="uuid",
                data=[self.uuid4() for _ in range(100000)],
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
