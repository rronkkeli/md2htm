<?php

function parse($markdown) {
    $sock = "unix:///run/mdserv/mdserv.sock";
    // Assumes big endianness
    $len = pack("J", strlen($markdown));
    $handle = fsockopen($sock);
    fwrite($handle, $len.$markdown);
    $len = fread($handle, 8);
    $len = unpack("J", $len);
    $html = fread($handle, $len[1]);
    fclose($handle);
    return $html;
}

?>
