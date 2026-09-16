from pathlib import Path

path = Path('src/movegen.rs')
text = path.read_text()
old = 'if fast_mode && target.location.above_stack(board)'
new = 'if fast_mode && target.spin == Spin::None && target.location.above_stack(board)'
assert text.count(old) == 1, 'Unexpected source revision'
path.write_text(text.replace(old, new))

# Use the same terminal loss value for explicit garbage overflow and for an
# expanded node without any legal continuation.
path = Path('src/bot/freestyle.rs')
text = path.read_text()
assert 'unwrap_or(-1000.0)' in text
path.write_text(text.replace('unwrap_or(-1000.0)', 'unwrap_or(-1_000_000.0)'))
