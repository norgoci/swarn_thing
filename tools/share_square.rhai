
    fn share_square(dummy) {
        let code = inspect_tool("square");
        return send_message("http://127.0.0.1:9998/message", code);
    }
    