use ldap3::{Ldap, LdapConnAsync, LdapError, Scope, SearchEntry};
use log::{trace, warn};

pub async fn connect_and_bind(url: &str, bind_dn: &str, password: &str) -> Result<Ldap, LdapError> {
    trace!("Connecting to LDAP: {}", url);

    let (conn, mut ldap) = LdapConnAsync::new(url).await?;
    ldap3::drive!(conn);

    trace!("Binding to LDAP as {}", bind_dn);
    ldap.simple_bind(bind_dn, password).await?.success()?;
    trace!("Bound to LDAP as {}", bind_dn);

    Ok(ldap)
}

pub async fn get_group_member_dns(
    ldap: &mut Ldap,
    group_dn: &str,
) -> Result<Vec<String>, LdapError> {
    trace!("Searching for group members in {}", group_dn);
    let (entries, _) = ldap
        .search(group_dn, Scope::Base, "(objectClass=*)", vec!["member"])
        .await?
        .success()?;

    trace!("Found {} entries", entries.len());

    let mut members = vec![];
    for entry in entries {
        let search = SearchEntry::construct(entry);
        if let Some(vals) = search.attrs.get("member") {
            trace!("Found {} members", vals.len());
            members.extend(vals.clone());
        }
    }

    Ok(members)
}

pub async fn resolve_uids(ldap: &mut Ldap, member_dns: &[String]) -> Vec<String> {
    let mut uids = Vec::new();

    for dn in member_dns {
        trace!("Searching for uid in {}", dn);

        if let Ok(result) = ldap
            .search(dn, Scope::Base, "(objectClass=*)", vec!["uid"])
            .await
        {
            if let Ok((entries, _)) = result.success() {
                trace!("Found {} entries", entries.len());
                for entry in entries {
                    let user = SearchEntry::construct(entry);
                    if let Some(vals) = user.attrs.get("uid") {
                        trace!("Resolved uid(s): {:?} for {}", vals, dn);
                        uids.extend(vals.clone());
                    }
                }
            }
        } else {
            warn!("Failed to search for uid in {}", dn);
        }
    }

    uids
}
