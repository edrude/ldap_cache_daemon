use ldap3::{Ldap, LdapConnAsync, LdapError, Scope, SearchEntry};
use log::{trace, warn};

fn parse_scope(s: &str) -> Result<Scope, String> {
    match s.to_lowercase().as_str() {
        "base" => Ok(Scope::Base),
        "subtree" => Ok(Scope::Subtree),
        _ => Err(format!("Invalid scope: {}", s)),
    }
}

pub async fn connect_and_bind(url: &str, bind_dn: &str, password: &str) -> Result<Ldap, LdapError> {
    trace!("Connecting to LDAP: {}", url);

    let (conn, mut ldap) = LdapConnAsync::new(url).await?;
    ldap3::drive!(conn);

    trace!("Binding to LDAP as {}", bind_dn);
    ldap.simple_bind(bind_dn, password).await?.success()?;
    trace!("Bound to LDAP as {}", bind_dn);

    Ok(ldap)
}

pub async fn query(
    ldap: &mut Ldap,
    base: &str,
    scope: &str,
    filter: &str,
    attr: &str,
) -> Result<Vec<String>, LdapError> {
    trace!("Search for '{}' in base '{}' with scope '{}'", filter, base, scope);
    let (results, _) = ldap.search(base, parse_scope(scope).unwrap(), filter, &[attr]).await?.success()?;
    // We should probably do a better job of handing edge cases. program is only designed to work
    // when a single entry is found. If no entries are found we may want to 404 instead of
    // returning an empty list
    match results.len() {
        0 => warn!("Found 0 entries for query, returning empty results, but you should know there is no entry in ldap"),
        n if n > 1 => warn!("Found more than one LDAP entry and we are only designed to look at one"),
        _ => trace!("Found 1 entry"),
    }


    let mut values = vec![];

    for result in results {
        let entry = SearchEntry::construct(result);
        if let Some(vals) = entry.attrs.get(attr) {
            values.extend(vals.clone());
        }
    }

    Ok(values)
}