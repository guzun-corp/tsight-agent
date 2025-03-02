use crate::config::{GlobalFilters, SqlFilterRules};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SqlFilters {
    // Exclude filters
    exclude_database_patterns: Vec<Regex>,
    exclude_table_patterns: Vec<Regex>,
    exclude_column_name_patterns: Vec<Regex>,
    exclude_column_value_patterns: Vec<Regex>,

    // Allow filters
    allow_database_patterns: Vec<Regex>,
    allow_table_patterns: Vec<Regex>,
    allow_column_name_patterns: Vec<Regex>,
    allow_column_value_patterns: Vec<Regex>,
}

impl SqlFilters {
    pub fn new(global_filters: Option<&GlobalFilters>) -> Result<Self, regex::Error> {
        let mut filters = SqlFilters {
            exclude_database_patterns: Vec::new(),
            exclude_table_patterns: Vec::new(),
            exclude_column_name_patterns: Vec::new(),
            exclude_column_value_patterns: Vec::new(),
            allow_database_patterns: Vec::new(),
            allow_table_patterns: Vec::new(),
            allow_column_name_patterns: Vec::new(),
            allow_column_value_patterns: Vec::new(),
        };

        if let Some(global_filters) = global_filters {
            // Process exclude filters
            if let Some(exclude_rules) = &global_filters.sql_filters_exclude {
                for rule in exclude_rules {
                    filters.add_exclude_patterns(rule)?;
                }
            }

            // Process allow filters
            if let Some(allow_rules) = &global_filters.sql_filters_allow {
                for rule in allow_rules {
                    filters.add_allow_patterns(rule)?;
                }
            }
        }

        Ok(filters)
    }

    fn add_exclude_patterns(&mut self, rules: &SqlFilterRules) -> Result<(), regex::Error> {
        if let Some(patterns) = &rules.database_regexes {
            for pattern in patterns {
                self.exclude_database_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.table_regexes {
            for pattern in patterns {
                self.exclude_table_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.column_name_regexes {
            for pattern in patterns {
                self.exclude_column_name_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.column_value_regexes {
            for pattern in patterns {
                self.exclude_column_value_patterns
                    .push(Regex::new(pattern)?);
            }
        }

        Ok(())
    }

    fn add_allow_patterns(&mut self, rules: &SqlFilterRules) -> Result<(), regex::Error> {
        if let Some(patterns) = &rules.database_regexes {
            for pattern in patterns {
                self.allow_database_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.table_regexes {
            for pattern in patterns {
                self.allow_table_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.column_name_regexes {
            for pattern in patterns {
                self.allow_column_name_patterns.push(Regex::new(pattern)?);
            }
        }

        if let Some(patterns) = &rules.column_value_regexes {
            for pattern in patterns {
                self.allow_column_value_patterns.push(Regex::new(pattern)?);
            }
        }

        Ok(())
    }

    pub fn should_exclude_database(&self, db_name: &str) -> bool {
        // If there are allow patterns and none match, exclude the database
        if !self.allow_database_patterns.is_empty() {
            let allowed = self
                .allow_database_patterns
                .iter()
                .any(|pattern| pattern.is_match(db_name));
            if !allowed {
                return true;
            }
        }

        // If any exclude pattern matches, exclude the database
        self.exclude_database_patterns
            .iter()
            .any(|pattern| pattern.is_match(db_name))
    }

    pub fn should_exclude_table(&self, table_name: &str) -> bool {
        // If there are allow patterns and none match, exclude the table
        if !self.allow_table_patterns.is_empty() {
            let allowed = self
                .allow_table_patterns
                .iter()
                .any(|pattern| pattern.is_match(table_name));
            if !allowed {
                return true;
            }
        }

        // If any exclude pattern matches, exclude the table
        self.exclude_table_patterns
            .iter()
            .any(|pattern| pattern.is_match(table_name))
    }

    pub fn should_exclude_column(&self, column_name: &str) -> bool {
        // If there are allow patterns and none match, exclude the column
        if !self.allow_column_name_patterns.is_empty() {
            let allowed = self
                .allow_column_name_patterns
                .iter()
                .any(|pattern| pattern.is_match(column_name));
            if !allowed {
                return true;
            }
        }

        // If any exclude pattern matches, exclude the column
        self.exclude_column_name_patterns
            .iter()
            .any(|pattern| pattern.is_match(column_name))
    }

    pub fn should_exclude_value(&self, value: &str) -> bool {
        // If there are allow patterns and none match, exclude the value
        if !self.allow_column_value_patterns.is_empty() {
            let allowed = self
                .allow_column_value_patterns
                .iter()
                .any(|pattern| pattern.is_match(value));
            if !allowed {
                return true;
            }
        }

        // If any exclude pattern matches, exclude the value
        self.exclude_column_value_patterns
            .iter()
            .any(|pattern| pattern.is_match(value))
    }
}
