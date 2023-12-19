use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launchers {
    EpicGamesLauncher,
    Legendary
}

pub fn launchers() -> [Launchers; 2] {
    [Launchers::EpicGamesLauncher, Launchers::Legendary]

}

impl Into<String> for Launchers {
    fn into(self) -> String {
        match self {
            Launchers::EpicGamesLauncher => String::from("EpicGamesLauncher"),
            Launchers::Legendary => String::from("Legendary"),
        }
    }
}

impl Display for Launchers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(self.clone()))?;

        Ok(())
    }

}

impl serde::Serialize for Launchers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            Launchers::EpicGamesLauncher => serializer.serialize_str("EpicGamesLauncher"),
            Launchers::Legendary => serializer.serialize_str("Legendary")
        }
    }
}

impl<'de> serde::Deserialize<'de> for Launchers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let launcher = String::deserialize(deserializer)?;

        match launcher.as_str() {
            "EpicGamesLauncher" => Ok(Launchers::EpicGamesLauncher),
            "Legendary" => Ok(Launchers::Legendary),
            _ => Err(serde::de::Error::custom("Invalid launcher"))
        }
    }
}