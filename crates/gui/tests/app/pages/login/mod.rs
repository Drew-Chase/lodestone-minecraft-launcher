use super::*;

#[test]
fn the_flow_moves_through_its_states() {
    let mut login = Login::default();
    assert_eq!(login.flow, Flow::Idle);

    login.update(Message::Opening);
    assert_eq!(login.flow, Flow::Opening);

    login.update(Message::CodeReady {
        user_code: "ABCD-EFGH".into(),
        verification_uri: "https://www.microsoft.com/link".into(),
    });
    assert_eq!(
        login.flow,
        Flow::Code {
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
            copied: false
        }
    );

    login.update(Message::CodeCopied);
    assert_eq!(
        login.flow,
        Flow::Code {
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
            copied: true
        }
    );

    login.update(Message::SignInFailed("the sign-in request was denied".into()));
    assert_eq!(login.flow, Flow::Failed("the sign-in request was denied".into()));

    login.update(Message::Reset);
    assert_eq!(login.flow, Flow::Idle);
}

#[test]
fn hover_flags_track_their_messages() {
    let mut login = Login::default();
    assert!(!login.code_hover && !login.link_hover);

    login.update(Message::CodeHover(true));
    login.update(Message::LinkHover(true));
    assert!(login.code_hover && login.link_hover);

    login.update(Message::CodeHover(false));
    assert!(!login.code_hover && login.link_hover);

    // Resetting the flow clears hover state too.
    login.update(Message::Reset);
    assert!(!login.code_hover && !login.link_hover);
}

#[test]
fn actions_are_the_roots_to_handle() {
    // The page never acts on its own action messages — the root
    // intercepts them (see the contract in `pages/mod.rs`).
    let mut login = Login::default();
    for message in [Message::SignInMicrosoft, Message::OfflineMode, Message::DemoMode, Message::LearnMore] {
        login.update(message.clone());
        assert_eq!(login.flow, Flow::Idle);
    }
    login.update(Message::Navigate(Page::Home));
    assert_eq!(login.flow, Flow::Idle);
}
