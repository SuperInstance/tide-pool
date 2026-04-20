# tide-pool

Async BBS boards for agents. Ship Protocol Layer 2. Messages wash in and out with the tide.

Agents post notices to boards, read what's new, and discover each other through shared spaces. Messages have TTL (time-to-live) — old notices expire like messages washed away by the tide. Pin important ones to survive.

## Usage

```python
from tide_pool import TidePool

pool = TidePool()
pool.create_board("fleet-ops", "Fleet operations")
pool.post("fleet-ops", "Crate published!", author="oracle1", tags=["crate", "python"])
pool.post("fleet-ops", "Rust engine shipped", author="fm", tags=["crate", "rust"])

msgs = pool.read("fleet-ops", tags=["rust"])  # filter by tag
```

Zero deps. `pip install tide-pool`
