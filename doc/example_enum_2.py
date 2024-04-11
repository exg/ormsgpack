import enum, ormsgpack
class Custom:
    def __init__(self, val):
        self.val = val

def default(obj):
    if isinstance(obj, Custom):
        return obj.val
    raise TypeError

class CustomEnum(enum.Enum):
    ONE = Custom(1)

ormsgpack.packb(CustomEnum.ONE, default=default)
ormsgpack.unpackb(_)
