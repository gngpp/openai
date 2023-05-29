use std::error::Error;

use base64::{engine::general_purpose, Engine};

fn main() {
    let splitted_jwt_strings: Vec<_> = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6Ik1UaEVOVUpHTkVNMVFURTRNMEZCTWpkQ05UZzVNRFUxUlRVd1FVSkRNRU13UmtGRVFrRXpSZyJ9.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiZ3JvdXBzIjpbXSwib3JnYW5pemF0aW9ucyI6W3siaWQiOiJvcmctWjZKMWtlaHd2OFZtSU0wZnh1eWR4U2JoIiwiaXNfZGVmYXVsdCI6dHJ1ZSwicm9sZSI6Im93bmVyIiwidGl0bGUiOiJQZXJzb25hbCJ9XSwidXNlcl9pZCI6InVzZXItNjZHS084dDBWOEwycjFLUDg4ZzV1Y3NNIn0sIm5pY2tuYW1lIjoiZ25ncHB4IiwibmFtZSI6ImduZ3BweEBnbWFpbC5jb20iLCJwaWN0dXJlIjoiaHR0cHM6Ly9zLmdyYXZhdGFyLmNvbS9hdmF0YXIvMjI0NTNlODI1ZWViYWM3YTQzYWRiZWFjMmQzZmYzNWY_cz00ODAmcj1wZyZkPWh0dHBzJTNBJTJGJTJGY2RuLmF1dGgwLmNvbSUyRmF2YXRhcnMlMkZnbi5wbmciLCJ1cGRhdGVkX2F0IjoiMjAyMy0wNS0yOVQxMzozMzo1NC41OTlaIiwiZW1haWwiOiJnbmdwcHhAZ21haWwuY29tIiwiZW1haWxfdmVyaWZpZWQiOnRydWUsImlzcyI6Imh0dHBzOi8vYXV0aDAub3BlbmFpLmNvbS8iLCJhdWQiOiJwZGxMSVgyWTcyTUlsMnJoTGhURTlWVjliTjkwNWtCaCIsImlhdCI6MTY4NTM2NzQyMiwiZXhwIjoxNjg1NDAzNDIyLCJzdWIiOiJhdXRoMHw2NDcxNWQ0YmYzNzU3N2I2Y2VkNjdjZjciLCJhdXRoX3RpbWUiOjE2ODUzNjcyMDR9.eDGW1kvfGJLFPIjMGwHl49lrUafVlBqDAATyNiLtbFVd4_u7P_sbIJ_CZr4IKhuWM2kUE-Iluajt3bX0owlOUrYJt0xQr98aXsIBEa3FAXe97Qekul5Wm1iHFysOzpLus84LBa1TDz-E55UcdsyoHER9OFbfauUBmYZ2JMH-VAOFrAkxDbV7XkGDcGu7rMEd01NjKzag_Pya9f3B0t6x2L2Eg26pyh3yaV0O-2ej9cP3Ug8KFZ10rcL9OdtBrQhhLHMWyPPxh6usJvNPK-EkJYFtMgHfag8WT1r7ts0Qb7tX5zwPn7K1dDi_7fRbAOC0gALgGnkgIiU241uflBGuSA".split('.').collect();

    let jwt_header = splitted_jwt_strings
        .get(0)
        .expect("split always returns at least one element");
    let jwt_body = splitted_jwt_strings
        .get(1)
        .ok_or(Box::<dyn Error>::from(
            "Could not find separator in jwt string.",
        ))
        .unwrap();

    let decoded_jwt_header = general_purpose::STANDARD.decode(jwt_header).unwrap();
    let decoded_jwt_body = general_purpose::URL_SAFE.decode(jwt_body).unwrap();

    let converted_jwt_header = String::from_utf8(decoded_jwt_header).unwrap();
    let converted_jwt_body = String::from_utf8(decoded_jwt_body).unwrap();

    let parsed_jwt_header =
        serde_json::from_str::<serde_json::Value>(&converted_jwt_header).unwrap();
    let parsed_jwt_body = serde_json::from_str::<OpenAIUserInfo>(&converted_jwt_body).unwrap();
    println!("{:?}", parsed_jwt_header);
    println!("{:?}", parsed_jwt_body)
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAIUserInfo {
    nickname: String,
    name: String,
    picture: String,
    updated_at: String,
    email: String,
    email_verified: bool,
    iss: String,
    aud: String,
    iat: i64,
    exp: i64,
    sub: String,
    auth_time: i64,
}
