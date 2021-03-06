// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheck {
    Ok,
    Warning,
    Critical,
    Unknown,
}

impl fmt::Display for HealthCheck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            HealthCheck::Ok => "OK",
            HealthCheck::Warning => "WARNING",
            HealthCheck::Critical => "CRITICAL",
            HealthCheck::Unknown => "UNKNOWN",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeCheck {
    Ok,
    Failed(i32),
    Pending,
}

impl fmt::Display for SmokeCheck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            SmokeCheck::Ok => "OK",
            SmokeCheck::Failed(_) => "FAILED",
            SmokeCheck::Pending => "PENDING",
        };
        write!(f, "{}", msg)
    }
}
