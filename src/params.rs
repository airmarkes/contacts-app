use serde::Deserialize;

//PARAMETERS FROM HTTP BODY FOR AXUM

#[derive(Deserialize)]
pub struct ShowParams {
    pub search_p: Option<String>,
    pub page_p: Option<u32>    
}

#[derive(Deserialize)]
pub struct NewContactParams {
    pub first_p: Option<String>, 
    pub last_p: Option<String>,
    pub phone_p: Option<String>,
    pub email_p: Option<String>,
    pub birth_p: Option<String>,     
} 

#[derive(Deserialize)]
pub struct ViewContactParams{
    pub id_p: Option<u32>
}

#[derive(Deserialize)]
pub struct EditContactParams{
    pub id_p: Option<u32>,
    pub first_p: Option<String>, 
    pub last_p: Option<String>,
    pub phone_p: Option<String>,
    pub email_p: Option<String>, 
    pub birth_p: Option<String>,  
}

#[derive(Deserialize)]
pub struct ValidateEmailParams{
    pub email_p: Option<String>,
    pub id_p: Option<u32>,
}

#[derive(Deserialize)]
pub struct DeleteBulkParams{
    #[serde(rename = "ids_p")]
    pub ids_p: Option<Vec<String>>,
}
