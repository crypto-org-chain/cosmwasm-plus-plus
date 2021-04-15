pub struct InitMsg {}

pub enum ExecuteMsg {
    CreatePlan(),
    StopPlan(),
    Subscribe(),
    Unsubscribe(),
    UnsubscribeUser(),
    TriggerCollection(),
}
