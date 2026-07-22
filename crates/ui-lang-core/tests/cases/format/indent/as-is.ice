app Demo
extern crate::backend
    Item(id:i64)
    load() -> [Item] ! Item
theme
    bg #000000
    fg #ffffff
    primary #333333
    danger #ff0000
state
    items:[Item] = []
on mount
    run load() -> loaded _ | failed _
on loaded(next)
    items = next
on failed(error)
    items = []
view
    text len(items) @text-sm
