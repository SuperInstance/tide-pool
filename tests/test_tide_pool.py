"""Tests for tide-pool."""
import time
import pytest
from tide_pool import TidePool, Message, MessageStatus


def test_post_and_read():
    pool = TidePool()
    pool.create_board("ops")
    pool.post("ops", "Crate published!", author="oracle1")
    msgs = pool.read("ops")
    assert len(msgs) == 1
    assert msgs[0].content == "Crate published!"


def test_read_marks_fresh():
    pool = TidePool()
    pool.create_board("test")
    pool.post("test", "hello", author="a")
    msgs = pool.read("test")
    assert msgs[0].status == MessageStatus.READ


def test_ttl_expiry():
    pool = TidePool()
    pool.create_board("old")
    # Post with very short TTL
    msg = pool.post("old", "expires fast", author="a", ttl=0.01)
    time.sleep(0.02)
    msgs = pool.read("old")
    assert len(msgs) == 0  # expired


def test_pinned_survives_expiry():
    pool = TidePool()
    pool.create_board("pin")
    pool.post("pin", "pinned msg", author="a", ttl=0.01)
    # Manually pin it
    board = pool.boards["pin"]
    for m in board._messages.values():
        m.status = MessageStatus.PINNED
    time.sleep(0.02)
    msgs = pool.read("pin")
    assert len(msgs) == 1


def test_tag_filtering():
    pool = TidePool()
    pool.create_board("fleet")
    pool.post("fleet", "rust crate", author="fm", tags=["rust", "crate"])
    pool.post("fleet", "python crate", author="o1", tags=["python", "crate"])
    pool.post("fleet", "no tags", author="a")
    
    rust_only = pool.read("fleet", tags=["rust"])
    assert len(rust_only) == 1
    assert rust_only[0].author == "fm"


def test_since_filter():
    pool = TidePool()
    pool.create_board("time")
    pool.post("time", "old", author="a")
    cutoff = time.time()
    pool.post("time", "new", author="b")
    msgs = pool.read("time", since=cutoff)
    assert len(msgs) == 1
    assert msgs[0].content == "new"


def test_subscriptions():
    pool = TidePool()
    pool.create_board("ops")
    pool.subscribe("ops", "oracle1")
    pool.subscribe("ops", "fm")
    boards = pool.boards_for_agent("oracle1")
    assert "ops" in boards
    stats = pool.stats()
    assert stats["total_subscribers"] == 2
