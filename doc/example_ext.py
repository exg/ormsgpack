import ormsgpack, decimal
def default(obj):
    if isinstance(obj, decimal.Decimal):
        return ormsgpack.Ext(0, str(obj).encode())
    raise TypeError

ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"), default=default)
