from __future__ import annotations

from typing import Any, Literal

import httpx
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field


RiskLevel = Literal["low", "medium", "high", "critical", "data_insufficient"]
SuggestedAction = Literal["allow", "review", "hold", "block"]


class OrderRiskWebhookRequest(BaseModel):
    order_id: str
    tenant_id: str
    shop_id: str
    features: dict[str, Any] = Field(default_factory=dict)


class OrderRiskWebhookResponse(BaseModel):
    risk_score: float = Field(ge=0, le=100)
    risk_level: RiskLevel
    reason: str
    suggested_action: SuggestedAction
    policy_flags: list[str] = Field(default_factory=list)
    requires_manual_review: bool


app = FastAPI(title="Order Risk Langflow Sidecar", version="0.1.0")


@app.post("/webhook/order-risk", response_model=OrderRiskWebhookResponse)
async def order_risk_webhook(payload: OrderRiskWebhookRequest) -> OrderRiskWebhookResponse:
    sanitized = payload.model_dump()
    sanitized["features"] = redact_features(payload.features)

    try:
        async with httpx.AsyncClient(timeout=3.0) as client:
            response = await client.post("https://LANGFLOW_HOST/api/v1/run/FLOW_ID", json=sanitized)
            response.raise_for_status()
    except httpx.HTTPError as exc:
        raise HTTPException(status_code=503, detail="langflow_unavailable") from exc

    return enforce_policy(OrderRiskWebhookResponse.model_validate(response.json()))


def redact_features(features: dict[str, Any]) -> dict[str, Any]:
    blocked = {"phone", "address", "id_number", "email", "token", "raw_payload"}
    return {key: value for key, value in features.items() if key.lower() not in blocked}


def enforce_policy(result: OrderRiskWebhookResponse) -> OrderRiskWebhookResponse:
    if result.risk_level == "low" and result.suggested_action in {"hold", "block"}:
        result.suggested_action = "review"
        result.requires_manual_review = True
        result.policy_flags.append("low_risk_action_downgraded_to_manual_review")
    if result.suggested_action in {"hold", "block"}:
        result.requires_manual_review = True
    return result
