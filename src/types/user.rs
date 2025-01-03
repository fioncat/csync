pub struct User {
    pub name: String,
    pub role_names: Vec<String>,
    pub create_time: u64,
    pub update_time: u64,

    pub roles: Option<Vec<Role>>,
    pub password: Option<Password>,
}

pub struct Role {
    pub name: String,
    pub rules: Vec<RoleRule>,

    pub create_time: u64,
    pub update_time: u64,
}

pub struct RoleRule {
    pub objects: Vec<String>,
    pub verbs: Vec<String>,
}

pub struct Password {
    pub salt: String,
    pub hash: String,
}
