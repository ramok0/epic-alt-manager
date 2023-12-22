#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthClient<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub secret: &'a str,
}

#[macro_export]
macro_rules! get_client {
    ($name:expr) => {
        crate::epic_clients::AuthClient::get($name).expect(&format!("Failed to get client {}", $name))
    };
}
impl AuthClient<'_> {
    fn new<'a>(id: &'a str, secret: &'a str, name: &'a str) -> AuthClient<'a> {
        AuthClient { id, name, secret }
    }

    pub fn clients() -> Vec<AuthClient<'static>> {
        vec![
            AuthClient::new("3446cd72694c4a4485d81b77adbb2141", "9209d4a5e25a457fb9b07489d313b41a", "fortniteIOSGameClient"),
            AuthClient::new("34a02cf8f4414e29b15921876da36f9a", "daafbccc737745039dffe53d94fc76cf", "launcherAppClient2"),
            AuthClient::new("3e13c5c57f594a578abe516eecb673fe", "530e316c337e409893c55ec44f22cd62", "UEFN"),
            AuthClient::new("3f69e56c7649492c8cc29f1af08a8a12", "b51ee9cb12234f50a69efa67ef53812e", "fortniteAndroidGameClient"),
            AuthClient::new("5229dcd3ac3845208b496649092f251b", "e3bd2d3e-bf8c-4857-9e7d-f3d947d220c7", "fortniteSwitchGameClient"),
            AuthClient::new("7a40f8cdafd346219a4a0a15522b8ed7", "a94578c3-3a79-4441-ad22-a4ef6c9380a1", "Epic Games Client Service"),
            AuthClient::new("98f7e42c2e3a4f86a74eb43fbb41ed39", "0a2449a2-001a-451e-afec-3e812901c4d7", "fortniteNewSwitchGameClient"),
            AuthClient::new("d8566f2e7f5c48f89683173eb529fee1", "255c7109c8274241986616e3702678b5", "fortnitePS4USGameClient"),
            AuthClient::new("ec684b8c687f479fadea3cb2ad83f5c6", "e1f31c211f28413186262d37a13fc84d", "fortnitePCGameClient"),
            AuthClient::new("efe3cbb938804c74b20e109d0efc1548", "6e31bdbae6a44f258474733db74f39ba", "fortniteCNGameClient"),
            AuthClient::new("xyza7891343Fr4ZSPkQZ3kaL3I2sX8B5", "F8BVRyHIqmct8cN9KSPbXsJszpiIZEYEFDiySxc1wuA", "Fortnite")
        ]
    }

    pub fn get(name:&str) -> Option<AuthClient<'static>> {
        AuthClient::clients().into_iter().find(|client| client.name == name)
    }
}