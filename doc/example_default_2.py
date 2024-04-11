import ormsgpack, decimal
def default(obj):
    if isinstance(obj, decimal.Decimal):
        return str(obj)

ormsgpack.packb({"set":{1, 2}}, default=default)
ormsgpack.unpackb(_)
