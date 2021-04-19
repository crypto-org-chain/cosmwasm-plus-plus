use cosmwasm_std::{Response, Uint128};
use cw0::Event;

pub struct SubscribeEvent<'a> {
    pub plan_id: Uint128,
    pub subscriber: &'a str,
}

impl<'a> Event for SubscribeEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.add_attribute("action", "subscribe");
        rsp.add_attribute("plan_id", self.plan_id);
        rsp.add_attribute("subscriber", self.subscriber);
    }
}

pub struct UnsubscribeEvent<'a> {
    pub plan_id: Uint128,
    pub subscriber: &'a str,
}

impl<'a> Event for UnsubscribeEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.add_attribute("action", "unsubscribe");
        rsp.add_attribute("plan_id", self.plan_id);
        rsp.add_attribute("subscriber", self.subscriber);
    }
}

pub struct CreatePlanEvent {
    pub plan_id: Uint128,
}

impl Event for CreatePlanEvent {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.add_attribute("action", "create-plan");
        rsp.add_attribute("plan_id", self.plan_id);
    }
}

pub struct StopPlanEvent {
    pub plan_id: Uint128,
}

impl Event for StopPlanEvent {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.add_attribute("action", "stop-plan");
        rsp.add_attribute("plan_id", self.plan_id);
    }
}
