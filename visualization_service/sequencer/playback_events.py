from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, TypeAdapter


class _EventBase(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)


class StepForward(_EventBase):
    type: Literal["step_forward"] = "step_forward"


class StepBack(_EventBase):
    type: Literal["step_back"] = "step_back"


class JumpTo(_EventBase):
    type: Literal["jump"] = "jump"
    step: int


class Reset(_EventBase):
    type: Literal["reset"] = "reset"


class SetSpeed(_EventBase):
    type: Literal["set_speed"] = "set_speed"
    multiplier: float


class SetPlaying(_EventBase):
    type: Literal["set_playing"] = "set_playing"
    value: bool


PlaybackEvent = Annotated[
    StepForward | StepBack | JumpTo | Reset | SetSpeed | SetPlaying,
    Field(discriminator="type"),
]

PLAYBACK_ADAPTER = TypeAdapter(PlaybackEvent)
