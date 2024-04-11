import ormsgpack, decimal
def default(obj):
    if isinstance(obj, decimal.Decimal):
        return str(obj)
    raise TypeError

ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"))
ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"), default=default)
ormsgpack.packb({1, 2}, default=default)
