import dataclasses
import enum
import random
import string
import uuid
from datetime import datetime, timedelta, timezone

import numpy
from numpy.typing import NDArray
from pydantic import BaseModel


@dataclasses.dataclass
class Group:
    name: str
    uid: uuid.UUID


class UserType(enum.Enum):
    User = 1
    Admin = 2
    System = 3


@dataclasses.dataclass
class User:
    active: bool
    created_time: datetime
    groups: list[Group]
    name: str
    score: float
    type: UserType
    uid: uuid.UUID


class GroupModel(BaseModel):
    name: str
    uid: uuid.UUID


class UserModel(BaseModel):
    active: bool
    created_time: datetime
    groups: list[GroupModel]
    name: str
    score: float
    type: UserType
    uid: uuid.UUID


class Generator:
    def __init__(self) -> None:
        self.random = random.Random(0)
        self.rng = numpy.random.default_rng(0)
        self.alphabets = (
            string.ascii_letters,
            "".join(chr(c) for c in range(0x3040, 0x309F)),
        )
        self.groups = [
            Group(
                name="daemon",
                uid=self.uuid4(),
            ),
            Group(
                name="mail",
                uid=self.uuid4(),
            ),
            Group(
                name="wheel",
                uid=self.uuid4(),
            ),
        ]
        self.min_time = datetime(1970, 1, 1, tzinfo=timezone.utc)
        self.max_time = datetime(2514, 1, 1, tzinfo=timezone.utc)

    def bool_array(self, size: int) -> NDArray[numpy.bool]:
        return self.rng.choice((True, False), size=size)

    def float_array(self, size: int) -> NDArray[numpy.float64]:
        return self.rng.random(size=size)

    def int_array(self, size: int) -> NDArray[numpy.int64]:
        return self.rng.integers(low=-(2**63), high=2**63, size=size)

    def datetime(self) -> datetime:
        high = int((self.max_time - self.min_time).total_seconds())
        return self.min_time + timedelta(seconds=self.random.randint(0, high))

    def string(self) -> str:
        return "".join(
            self.random.choices(
                self.random.choice(self.alphabets),
                k=self.random.randint(1, 64),
            )
        )

    def uuid4(self) -> uuid.UUID:
        return uuid.UUID(bytes=self.random.randbytes(16), version=4)

    def user(self) -> User:
        name = self.string()
        return User(
            active=self.random.choice((True, False)),
            created_time=self.datetime(),
            groups=[
                Group(
                    name=name,
                    uid=self.uuid4(),
                ),
                *self.random.sample(
                    self.groups,
                    k=self.random.randint(1, len(self.groups)),
                ),
            ],
            name=name,
            score=self.random.random(),
            type=self.random.choice(list(UserType)),
            uid=self.uuid4(),
        )


def datasets(generator: Generator) -> dict[str, object]:
    user_dataclasses = [generator.user() for _ in range(1000)]
    user_dicts = [dataclasses.asdict(user) for user in user_dataclasses]
    user_models = [UserModel(**user) for user in user_dicts]
    bool_array = generator.bool_array(100_000)
    float_array = generator.float_array(100_000)
    int_array = generator.int_array(100_000)
    return {
        "dataclass": user_dataclasses,
        "dict": user_dicts,
        "pydantic": user_models,
        "bool": bool_array.tolist(),
        "float": float_array.tolist(),
        "int": int_array.tolist(),
        "numpy.bool": bool_array,
        "numpy.float": float_array,
        "numpy.int": int_array,
        "datetime": [generator.datetime() for _ in range(10_000)],
        "uuid": [generator.uuid4() for _ in range(10_000)],
    }
