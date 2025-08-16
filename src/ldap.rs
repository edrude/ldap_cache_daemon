use ldap3::{Ldap, LdapConnAsync, LdapError, Scope, SearchEntry};
use log::{trace, warn, debug};

fn parse_scope(s: &str) -> Result<Scope, String> {
    match s.to_lowercase().as_str() {
        "base" => Ok(Scope::Base),
        "subtree" => Ok(Scope::Subtree),
        _ => Err(format!("Invalid scope: {}", s)),
    }
}

pub async fn connect_and_bind(url: &str, bind_dn: &str, password: &str) -> Result<Ldap, LdapError> {
    debug!("Connecting to LDAP server: {}", url);

    let (conn, mut ldap) = LdapConnAsync::new(url).await?;
    ldap3::drive!(conn);
    debug!("LDAP connection established successfully");

    trace!("Binding to LDAP as {}", bind_dn);
    ldap.simple_bind(bind_dn, password).await?.success()?;
    debug!("Successfully bound to LDAP as {}", bind_dn);

    Ok(ldap)
}

pub async fn query(
    ldap: &mut Ldap,
    base: &str,
    scope: &str,
    filter: &str,
    attr: &str,
) -> Result<Vec<String>, LdapError> {
    debug!("Executing LDAP search: base='{}', scope='{}', filter='{}', attr='{}'", base, scope, filter, attr);
    
    let (results, _) = ldap.search(base, parse_scope(scope).unwrap(), filter, &[attr]).await?.success()?;
    
    // Log the number of results found
    match results.len() {
        0 => {
            warn!("Found 0 entries for query, returning empty results");
            debug!("Query details: base='{}', filter='{}', attr='{}'", base, filter, attr);
        },
        n if n > 1 => {
            warn!("Found {} LDAP entries, only designed to handle single entries", n);
            debug!("Multiple results found for: base='{}', filter='{}'", base, filter);
        },
        _ => {
            debug!("Found 1 LDAP entry as expected");
        }
    }

    let mut values = vec![];

    for result in results {
        let entry = SearchEntry::construct(result);
        if let Some(vals) = entry.attrs.get(attr) {
            values.extend(vals.clone());
            trace!("Extracted {} values from LDAP entry", vals.len());
        } else {
            debug!("No values found for attribute '{}' in LDAP entry", attr);
        }
    }

    debug!("LDAP query completed: found {} values for attribute '{}'", values.len(), attr);
    Ok(values)
}