// use mantra_lang_tracing::extract_req_ids_from_str;
// use mantra_rust_macros::req;

// #[test]
// #[req(req_id)]
// fn single_req() {
//     let req = "req_id";
//     let reqs = extract_req_ids_from_str(req).unwrap();

//     assert_eq!(
//         &reqs.first().unwrap(),
//         &req,
//         "Single requirement ID not extracted correctly."
//     );
//     assert_eq!(
//         reqs.len(),
//         1,
//         "More/Less than one requirement ID extracted."
//     );
// }

// #[test]
// #[req(trace.multiple)]
// fn multiple_reqs() {
//     let trace_content = "first_id, second_id";
//     let reqs = extract_req_ids_from_str(trace_content).unwrap();

//     assert_eq!(
//         reqs.first().unwrap(),
//         "first_id",
//         "First requirement ID not extracted correctly."
//     );
//     assert_eq!(
//         reqs.last().unwrap(),
//         "second_id",
//         "Second requirement ID not extracted correctly."
//     );
//     assert_eq!(
//         reqs.len(),
//         2,
//         "More/Less than two requirement ID extracted."
//     );
// }

// #[test]
// #[req(trace.special_chars)]
// fn quoted_id() {
//     let trace_content = "\"req-id.sub-req\"";
//     let reqs = extract_req_ids_from_str(trace_content).unwrap();

//     assert_eq!(
//         reqs.first().unwrap(),
//         "req-id.sub-req",
//         "Quoted requirement ID not extracted correctly."
//     );
//     assert_eq!(
//         reqs.len(),
//         1,
//         "More/Less than one requirement ID extracted."
//     );
// }

// #[test]
// #[req(trace.special_chars)]
// fn invalid_id() {
//     let trace_content = "invalid`char";
//     let reqs = extract_req_ids_from_str(trace_content);

//     assert!(reqs.is_err(), "Invalid char in ID extracted without error.");
// }
