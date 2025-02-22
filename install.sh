#!/bin/bash
cp ./md2htm /usr/bin/

echo "Do you want to enable the daemon service? (y/n) "
read ans

if [[ "$ans" == "y" ]]
then
    cp ./md2htm.service /lib/systemd/system/
    systemctl enable md2htm.service
    echo "Service enabled!"
fi

echo "Done!"
