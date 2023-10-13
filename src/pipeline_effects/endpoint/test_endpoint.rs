use super::{basenames, construct_endpoint_url};
use rand;

#[test]
fn test_construct_endpoint_url_test_happy_path() {
    let dummy_endpoint = "https://api.hotpotato.com/api/v1/foo";
    let extension = vec![
        format!("bar{}", rand::random::<u32>()),
        format!("hello{}", rand::random::<u32>()),
        format!("world{}", rand::random::<u32>()),
    ];
    let expected = format!(
        "{}/{}/{}/{}",
        dummy_endpoint, extension[0], extension[1], extension[2]
    );
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(expected, output);
}

#[test]
fn test_construct_endpoint_url_test_no_segments_no_slash() {
    let dummy_endpoint = "https://api.hotpotato.com";
    let extension = vec![
        format!("bar{}", rand::random::<u32>()),
        format!("hello{}", rand::random::<u32>()),
        format!("world{}", rand::random::<u32>()),
    ];
    let expected = format!(
        "{}/{}/{}/{}",
        dummy_endpoint, extension[0], extension[1], extension[2]
    );
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(expected, output);
}

#[test]
fn test_construct_endpoint_url_test_no_segments_with_slash() {
    let prefix = "https://api.hotpotato.com";
    let dummy_endpoint = &format!("{}/", prefix);
    let extension = vec![
        format!("bar{}", rand::random::<u32>()),
        format!("hello{}", rand::random::<u32>()),
        format!("world{}", rand::random::<u32>()),
    ];
    let expected = format!(
        "{}/{}/{}/{}",
        prefix, extension[0], extension[1], extension[2]
    );
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(expected, output);
}

#[test]
fn test_construct_endpoint_url_test_empty_extension_some_segments() {
    let dummy_endpoint = "https://api.hotpotato.com/foo/bar";
    let extension = vec![];
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(dummy_endpoint, output);
}

#[test]
fn test_construct_endpoint_url_test_empty_extension_no_segments_no_slash() {
    let dummy_endpoint = "https://api.hotpotato.com";
    let extension = vec![];
    let expected = format!("{}/", dummy_endpoint); // will avoid redirects
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(expected, output);
}

#[test]
fn test_construct_endpoint_url_test_empty_extension_no_segments_no_slash_not_http() {
    let dummy_endpoint = "blah://api.hotpotato.com";
    let extension = vec![];
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(dummy_endpoint, output);
}

#[test]
fn test_construct_endpoint_url_test_empty_extension_no_segments_with_slash() {
    let dummy_endpoint = "https://api.hotpotato.com/";
    let extension = vec![];
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(dummy_endpoint, output);
}

#[test]
fn test_construct_endpoint_url_test_extra_trailing_slashes() {
    let prefix = "https://api.hotpotato.com";
    let dummy_endpoint = &format!("{}/////", prefix);
    let extension = vec![
        format!("bar{}", rand::random::<u32>()),
        format!("hello{}", rand::random::<u32>()),
        format!("world{}", rand::random::<u32>()),
    ];
    let expected = format!(
        "{}/{}/{}/{}",
        prefix, extension[0], extension[1], extension[2]
    );
    let output = construct_endpoint_url(dummy_endpoint, &extension).unwrap();
    assert_eq!(expected, output);
}

#[test]
fn test_basenames_test_happy_path() {
    let base_names = vec![
        format!("foo{}", rand::random::<u32>()),
        format!("bar{}", rand::random::<u32>()),
        format!("a{}", rand::random::<u32>()),
    ];
    let paths = base_names
        .iter()
        .map(|basename| format!("some/random/directory/{}", basename))
        .collect();
    let output = basenames(&paths);
    for (i, base_name) in output.iter().enumerate() {
        assert_eq!(&base_names[i], base_name); // order is preserved
    }
}

#[test]
fn test_basenames_test_empty_list() {
    let base_names: Vec<String> = vec![];
    let output = basenames(&base_names);
    assert_eq!(output.len(), 0);
}

#[test]
fn test_basenames_test_no_slashes() {
    let base_names = vec![
        format!("foo{}", rand::random::<u32>()),
        format!("bar{}", rand::random::<u32>()),
        format!("a{}", rand::random::<u32>()),
    ];
    let output = basenames(&base_names);
    for (i, base_name) in output.iter().enumerate() {
        assert_eq!(&base_names[i], base_name);
    }
}

#[test]
fn test_basenames_test_empty_strings() {
    let base_names = vec![format!(""), format!("")];
    let output = basenames(&base_names);
    for (i, base_name) in output.iter().enumerate() {
        assert_eq!(&base_names[i], base_name);
    }
}

#[test]
fn test_basenames_test_mixed_case() {
    let mut base_names = vec![
        format!("foo{}", rand::random::<u32>()),
        format!("bar{}", rand::random::<u32>()),
        format!("a{}", rand::random::<u32>()),
    ];
    let mut paths: Vec<String> = base_names
        .iter()
        .map(|basename| format!("some/random/directory/{}", basename))
        .collect();
    paths.push("".to_string());
    base_names.push("".to_string());
    let x = format!("b{}", rand::random::<u32>());
    paths.push(format!("{}", &x));
    base_names.push(format!("{}", &x));
    let output = basenames(&paths);
    for (i, base_name) in output.iter().enumerate() {
        assert_eq!(&base_names[i], base_name);
    }
}
