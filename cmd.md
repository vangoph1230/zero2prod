curl -i -X POST -d 'email=thomasd_manjvn@hotmail.com&name=Todjmv' http://127.0.0.1:8000/subscriptions

curl --request POST --data 'name=le%20guin&email=ursula_le_guin%40gmail.com'  https://zero2prod-d3zbe.ondigitalocean.app/subscriptions --verbose

chmod u+x init_db.sh

--------------------------------------------------------------------------------
DigitalOcean云操作步骤：
1、创建应用程序：
    doctl apps create --spec spec.yaml
2、doctl认证：
    doctl auth init
3、检查应用程序状态：
    doctl apps list
4、推送到Github后，触发新的部署：
    doctl apps update APP-ID --spec spec.yaml

----------------------------------------------------------------------------------

