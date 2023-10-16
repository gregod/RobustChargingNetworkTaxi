#! /bin/awk -f


# add header + footer
BEGIN {
    best=99999;
    print "digraph D {";
    print "\" puid-0-0 \" -> {\" puid-0-3 \",\" puid-0-1 \",\" puid-0-2 \"}";
}

END { print "}" }

# branch nodes after finish
/^NODE/ {

    # mark new bests
    if($5<best && $5 > 0){
        best=$5;st="style=filled,fillcolor=\"red\","
    }

    # draw node
     print "\"",$2,"\"[",st,"shape=record,label=\"",$2,"|{",$4,"|",$5,"}\"]";st=""
    # with type
    # print "\"",$2,"\"[",st,"shape=record,label=\"{",$2,"|",$7,"}|{",$4,"|",$5,"}\"]";st=""
    # with state
    # print "\"",$2,"\"[",st,"shape=record,label=\"",$2,"|{",$4,"|",$5,"}|",$6,"\"]";st=""

}

# branches from parent
/^CHILD/ {
    print "\"",$3,"\"-> \"",$2,"\" ";
}
