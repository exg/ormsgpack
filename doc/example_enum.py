import enum, datetime, ormsgpack
class DatetimeEnum(enum.Enum):
    EPOCH = datetime.datetime(1970, 1, 1, 0, 0, 0)

ormsgpack.packb(DatetimeEnum.EPOCH)
ormsgpack.unpackb(_)
ormsgpack.packb(DatetimeEnum.EPOCH, option=ormsgpack.OPT_NAIVE_UTC)
ormsgpack.unpackb(_)
