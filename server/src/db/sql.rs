use csync_misc::api::{QueryRequest, Value};

pub struct Select {
    fields: Vec<&'static str>,
    table: &'static str,

    wheres: Vec<String>,

    limit: bool,
    offset: bool,

    order_by: Vec<&'static str>,

    values: Vec<Value>,

    count: bool,
}

impl Select {
    pub fn new(fields: Vec<&'static str>, table: &'static str) -> Self {
        Self {
            fields,
            table,
            wheres: Vec::new(),
            limit: false,
            offset: false,
            order_by: Vec::new(),
            values: Vec::new(),
            count: false,
        }
    }

    pub fn count(table: &'static str) -> Self {
        Self {
            fields: vec!["COUNT(1)"],
            table,
            wheres: Vec::new(),
            limit: false,
            offset: false,
            order_by: Vec::new(),
            values: Vec::new(),
            count: true,
        }
    }

    pub fn add_order_by(&mut self, s: &'static str) {
        if self.count {
            return;
        }
        self.order_by.push(s);
    }

    pub fn add_where(&mut self, s: impl ToString, value: Value) {
        self.wheres.push(s.to_string());
        self.values.push(value);
    }

    pub fn set_query(&mut self, query: QueryRequest, search_field: &str) {
        if let Some(search) = query.search {
            let search = format!("%{search}%");
            self.add_where(format!("{search_field} LIKE ?"), Value::Text(search));
        }

        if let Some(update_after) = query.update_after {
            self.add_where("update_time > ?", Value::Integer(update_after));
        }

        if let Some(update_before) = query.update_before {
            self.add_where("update_time < ?", Value::Integer(update_before));
        }

        if self.count {
            return;
        }

        if let Some(limit) = query.limit {
            self.limit = true;
            self.values.push(Value::Integer(limit));
            if let Some(offset) = query.offset {
                self.offset = true;
                self.values.push(Value::Integer(offset));
            }
        }
    }

    pub fn build(self) -> (String, Vec<Value>) {
        let mut sql = format!("SELECT {} FROM {}", self.fields.join(", "), self.table);

        if !self.wheres.is_empty() {
            let where_clause = self.wheres.join(" AND ");
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        if !self.order_by.is_empty() {
            let order_by = self.order_by.join(", ");
            sql.push_str(&format!(" ORDER BY {}", order_by));
        }

        if self.limit {
            sql.push_str(" LIMIT ?");
            if self.offset {
                sql.push_str(" OFFSET ?");
            }
        }

        (sql, self.values)
    }
}

pub struct Update {
    table: &'static str,

    fields: Vec<&'static str>,
    wheres: Vec<String>,
    values: Vec<Value>,
}

impl Update {
    pub fn new(table: &'static str) -> Self {
        Self {
            table,
            fields: Vec::new(),
            wheres: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: &'static str, value: Value) {
        self.fields.push(field);
        self.values.push(value);
    }

    pub fn add_where(&mut self, s: impl ToString, value: Value) {
        self.wheres.push(s.to_string());
        self.values.push(value);
    }

    pub fn build(self) -> (String, Vec<Value>) {
        if self.fields.is_empty() {
            return (String::new(), Vec::new());
        }
        let mut sql = format!("UPDATE {} SET ", self.table);
        let set = self
            .fields
            .iter()
            .map(|f| format!("{} = ?", f))
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&set);

        if !self.wheres.is_empty() {
            let where_clause = self.wheres.join(" AND ");
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        (sql, self.values)
    }
}
